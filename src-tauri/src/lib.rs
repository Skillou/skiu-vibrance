use std::{
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use serde::Serialize;
use skiu_core::{load_settings, save_settings, Engine, GpuInfo, Runtime, Settings, WindowInfo};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, RunEvent, State, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_dialog::DialogExt;

/// Frequence de detection du jeu au premier plan.
const POLL_EVERY: Duration = Duration::from_millis(300);

/// Etat partage entre les commandes, le thread de surveillance et le menu de la zone de notification.
struct Shared {
    engine: Mutex<Engine>,
    path: PathBuf,
    pause_item: Mutex<Option<CheckMenuItem<tauri::Wry>>>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Engine> {
        self.engine.lock().unwrap_or_else(|p| p.into_inner())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StateDto {
    settings: Settings,
    gpu: GpuInfo,
    runtime: Runtime,
    autostart: bool,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Picked {
    name: String,
    exe: String,
    path: String,
}

/// Texte de l'infobulle de la zone de notification, a jour avec l'etat courant.
fn tray_tooltip(runtime: &Runtime, paused: bool) -> String {
    let level = runtime
        .applied_percent
        .map(|p| format!(" · {p} %"))
        .unwrap_or_default();
    let state = if runtime.error.is_some() {
        "erreur pilote".to_string()
    } else if runtime.previewing {
        format!("aperçu{level}")
    } else if paused {
        "en pause".to_string()
    } else if let Some(name) = &runtime.active_game_name {
        format!("{name}{level}")
    } else {
        format!("bureau{level}")
    };
    format!("Skiu Vibrance — {state}")
}

fn refresh_tray(app: &AppHandle, runtime: &Runtime, paused: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tray_tooltip(runtime, paused)));
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Applique de nouveaux reglages ; `notify` previent aussi l'interface (cas du menu de la zone de notification).
fn apply_settings(app: &AppHandle, shared: &Shared, settings: Settings, notify: bool) -> Runtime {
    let paused = settings.paused;
    let (runtime, current) = {
        let mut e = shared.lock();
        let rt = e.update_settings(settings, Instant::now());
        (rt, e.settings().clone())
    };
    if let Some(item) = shared.pause_item.lock().ok().and_then(|g| g.clone()) {
        let _ = item.set_checked(paused);
    }
    refresh_tray(app, &runtime, paused);
    if notify {
        let _ = app.emit("settings", &current);
        let _ = app.emit("runtime", &runtime);
    }
    runtime
}

#[tauri::command]
fn get_state(app: AppHandle, shared: State<'_, Arc<Shared>>) -> StateDto {
    let e = shared.lock();
    StateDto {
        settings: e.settings().clone(),
        gpu: e.gpu(),
        runtime: e.runtime().clone(),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
fn update_settings(app: AppHandle, shared: State<'_, Arc<Shared>>, settings: Settings) -> Runtime {
    apply_settings(&app, &shared, settings, false)
}

#[tauri::command]
fn set_preview(app: AppHandle, shared: State<'_, Arc<Shared>>, on: bool) -> Runtime {
    let (runtime, paused) = {
        let mut e = shared.lock();
        (e.set_preview(on, Instant::now()), e.settings().paused)
    };
    refresh_tray(&app, &runtime, paused);
    runtime
}

#[tauri::command]
fn list_windows(shared: State<'_, Arc<Shared>>) -> Vec<WindowInfo> {
    shared.lock().list_windows()
}

#[tauri::command]
async fn pick_exe(app: AppHandle) -> Option<Picked> {
    tauri::async_runtime::spawn_blocking(move || {
        let file = app
            .dialog()
            .file()
            .set_title("Choisir le jeu (.exe)")
            .add_filter("Programmes", &["exe"])
            .blocking_pick_file()?;
        let path = file.into_path().ok()?;
        Some(Picked {
            name: path.file_stem()?.to_string_lossy().to_string(),
            exe: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
        })
    })
    .await
    .ok()
    .flatten()
}

/// Icone d'un executable (data URL PNG), ou `None` si introuvable.
#[tauri::command]
async fn exe_icon(path: String) -> Option<String> {
    let p = std::path::Path::new(&path);
    if !p.is_absolute() || !p.is_file() {
        return None;
    }
    tauri::async_runtime::spawn_blocking(move || skiu_win::exe_icon_data_url(&path))
        .await
        .ok()
        .flatten()
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| e.to_string())?;
    manager.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Surveille la fenetre au premier plan et enregistre les reglages modifies.
fn spawn_monitor(app: AppHandle, shared: Arc<Shared>) {
    std::thread::spawn(move || loop {
        let (runtime, changed, to_save, paused, learned) = {
            let mut e = shared.lock();
            let now = Instant::now();
            let (rt, changed) = e.tick(now);
            let learned = e.take_learned().then(|| e.settings().clone());
            (rt, changed, e.take_dirty(now), e.settings().paused, learned)
        };
        if let Some(s) = learned {
            let _ = app.emit("settings", &s);
        }
        if changed {
            let _ = app.emit("runtime", &runtime);
            refresh_tray(&app, &runtime, paused);
        }
        if let Some(s) = to_save {
            let _ = save_settings(&shared.path, &s);
        }
        std::thread::sleep(POLL_EVERY);
    });
}

fn build_tray(app: &AppHandle, shared: &Arc<Shared>) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Ouvrir Skiu Vibrance", true, None::<&str>)?;
    let paused = shared.lock().settings().paused;
    let pause = CheckMenuItem::with_id(
        app,
        "pause",
        "Mettre la bascule en pause",
        true,
        paused,
        None::<&str>,
    )?;
    let update = MenuItem::with_id(
        app,
        "update",
        "Rechercher les mises à jour…",
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &pause, &update, &sep, &quit])?;
    if let Ok(mut slot) = shared.pause_item.lock() {
        *slot = Some(pause);
    }

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Skiu Vibrance")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "pause" => {
                if let Some(shared) = app.try_state::<Arc<Shared>>() {
                    let mut s = shared.lock().settings().clone();
                    s.paused = !s.paused;
                    apply_settings(app, &shared, s, true);
                }
            }
            "update" => {
                show_main(app);
                let _ = app.emit("check-update", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app)
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("settings.json");
            let saved = load_settings(&path);
            let first_run = saved.is_none();
            let (dvc, sys) = skiu_win::platform();
            let mut engine = Engine::new(saved.unwrap_or_default(), dvc, sys);
            if first_run {
                engine.adopt_current_as_desktop();
            }
            let shared = Arc::new(Shared {
                engine: Mutex::new(engine),
                path,
                pause_item: Mutex::new(None),
            });
            app.manage(shared.clone());
            build_tray(app.handle(), &shared)?;
            spawn_monitor(app.handle().clone(), shared);

            // Lance par Windows au demarrage : on reste discret dans la zone de notification.
            if !std::env::args().any(|a| a == "--minimized") {
                show_main(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            update_settings,
            set_preview,
            list_windows,
            pick_exe,
            exe_icon,
            set_autostart,
            quit_app
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if let Some(shared) = window.app_handle().try_state::<Arc<Shared>>() {
                    if shared.lock().settings().close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("erreur au demarrage de Skiu Vibrance")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                if let Some(shared) = app.try_state::<Arc<Shared>>() {
                    let mut e = shared.lock();
                    if let Some(s) = e.flush() {
                        let _ = save_settings(&shared.path, &s);
                    }
                    e.restore();
                }
            }
        });
}
