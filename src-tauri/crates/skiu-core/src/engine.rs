use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    model::{DisplayId, Game, GpuInfo, Runtime, Scope, Settings, WindowInfo},
    traits::{Dvc, Sys},
};

/// Duree maximale de l'apercu « en jeu » (securite si l'utilisateur oublie de l'arreter).
pub const PREVIEW_MAX: Duration = Duration::from_secs(12);
/// Delai sans modification avant d'ecrire les reglages sur le disque.
pub const SAVE_DEBOUNCE: Duration = Duration::from_millis(600);
/// Intervalle de verification du niveau reel du pilote : certains jeux (plein ecran
/// exclusif) ou le pilote lui-meme remettent la vibrance a zero dans notre dos.
pub const RESYNC_EVERY: Duration = Duration::from_secs(3);
/// Ecart tolere (en %) entre le niveau lu et le niveau voulu (arrondis de l'echelle native).
const RESYNC_TOLERANCE: i32 = 1;

pub struct Engine {
    settings: Settings,
    dvc: Box<dyn Dvc>,
    sys: Box<dyn Sys>,
    applied: HashMap<DisplayId, i32>,
    runtime: Runtime,
    preview_until: Option<Instant>,
    dirty_at: Option<Instant>,
    last_error: Option<String>,
    last_resync: Option<Instant>,
    /// Plage reellement pilotable (en %), si le pilote est disponible.
    range: Option<(i32, i32)>,
}

/// Jeu detecte au premier plan (ou apercu), avec le niveau a appliquer.
struct GameHit {
    game: Option<Game>,
    percent: i32,
    monitor: Option<String>,
}

impl Engine {
    pub fn new(settings: Settings, dvc: Box<dyn Dvc>, sys: Box<dyn Sys>) -> Self {
        let info = dvc.info();
        let range = (info.available && info.max_percent > info.min_percent)
            .then_some((info.min_percent, info.max_percent));
        let mut me = Self {
            settings,
            dvc,
            sys,
            applied: HashMap::new(),
            runtime: Runtime::default(),
            preview_until: None,
            dirty_at: None,
            last_error: None,
            last_resync: None,
            range,
        };
        let before = me.settings.clone();
        me.normalize();
        if me.settings != before {
            // Reglages corriges (bornes, doublons) : on les reecrit au prochain cycle.
            me.dirty_at = Some(Instant::now());
        }
        me
    }

    fn normalize(&mut self) {
        self.settings.sanitize();
        if let Some((lo, hi)) = self.range {
            self.settings.clamp_to(lo, hi);
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn gpu(&self) -> GpuInfo {
        self.dvc.info()
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn list_windows(&mut self) -> Vec<WindowInfo> {
        self.sys.windows()
    }

    /// Premier lancement : reprend le niveau actuel du pilote comme niveau « bureau »
    /// pour ne rien changer a l'ecran de l'utilisateur au demarrage.
    pub fn adopt_current_as_desktop(&mut self) {
        let first = self.dvc.displays().into_iter().next();
        if let Some(d) = first {
            if let Ok(p) = self.dvc.get_percent(d) {
                self.settings.desktop = p;
                self.normalize();
                self.dirty_at = Some(Instant::now());
            }
        }
    }

    pub fn update_settings(&mut self, settings: Settings, now: Instant) -> Runtime {
        self.settings = settings;
        self.normalize();
        self.dirty_at = Some(now);
        self.step(now);
        self.runtime.clone()
    }

    pub fn set_preview(&mut self, on: bool, now: Instant) -> Runtime {
        self.preview_until = if on { Some(now + PREVIEW_MAX) } else { None };
        self.step(now);
        self.runtime.clone()
    }

    /// Un cycle de surveillance. Renvoie l'etat courant et s'il a change.
    pub fn tick(&mut self, now: Instant) -> (Runtime, bool) {
        let before = self.runtime.clone();
        if self
            .last_resync
            .is_none_or(|t| now.duration_since(t) >= RESYNC_EVERY)
        {
            self.last_resync = Some(now);
            self.resync();
        }
        self.step(now);
        let changed = self.runtime != before;
        (self.runtime.clone(), changed)
    }

    /// Reglages a ecrire si ils sont restes stables assez longtemps.
    pub fn take_dirty(&mut self, now: Instant) -> Option<Settings> {
        match self.dirty_at {
            Some(t) if now.duration_since(t) >= SAVE_DEBOUNCE => {
                self.dirty_at = None;
                Some(self.settings.clone())
            }
            _ => None,
        }
    }

    /// Ecriture immediate (fermeture de l'application).
    pub fn flush(&mut self) -> Option<Settings> {
        self.dirty_at.take().map(|_| self.settings.clone())
    }

    /// Remet le niveau « bureau » partout (fermeture de l'application).
    /// Ecrit sans se fier au cache : le niveau a pu etre modifie par un autre programme.
    pub fn restore(&mut self) {
        if !self.settings.restore_on_exit {
            return;
        }
        let target = self.settings.desktop;
        for d in self.dvc.displays() {
            let _ = self.dvc.set_percent(d, target);
        }
        self.applied.clear();
    }

    /// Oublie les niveaux que le pilote n'a plus (modifies par un jeu, le Panneau NVIDIA,
    /// un changement de resolution...) pour qu'ils soient reappliques au prochain cycle.
    fn resync(&mut self) {
        let known: Vec<(DisplayId, i32)> = self.applied.iter().map(|(&d, &p)| (d, p)).collect();
        for (d, wanted) in known {
            match self.dvc.get_percent(d) {
                Ok(actual) if (actual - wanted).abs() <= RESYNC_TOLERANCE => {}
                _ => {
                    self.applied.remove(&d);
                }
            }
        }
    }

    fn matching_game(&mut self, now: Instant) -> Option<GameHit> {
        if let Some(t) = self.preview_until {
            if now >= t {
                self.preview_until = None;
            }
        }
        if self.preview_until.is_some() {
            return Some(GameHit {
                game: None,
                percent: self.settings.game_default,
                monitor: None,
            });
        }
        if self.settings.paused {
            return None;
        }
        let fg = self.sys.foreground()?;
        let game = self
            .settings
            .games
            .iter()
            .find(|g| g.enabled && g.exe.eq_ignore_ascii_case(&fg.exe))?
            .clone();
        let percent = game.percent.unwrap_or(self.settings.game_default);
        Some(GameHit {
            game: Some(game),
            percent,
            monitor: fg.monitor,
        })
    }

    fn step(&mut self, now: Instant) {
        let hit = self.matching_game(now);
        let displays = self.dvc.displays();
        self.applied.retain(|d, _| displays.contains(d));

        // Ecran cible quand le reglage « seulement l'ecran du jeu » est actif.
        let game_display = match (&hit, self.settings.scope) {
            (Some(h), Scope::GameMonitor) => h
                .monitor
                .as_deref()
                .and_then(|m| self.dvc.display_for_monitor(m)),
            _ => None,
        };

        for &d in &displays {
            let target = match &hit {
                Some(h) if game_display.is_none() || game_display == Some(d) => h.percent,
                _ => self.settings.desktop,
            };
            if self.applied.get(&d) != Some(&target) {
                match self.dvc.set_percent(d, target) {
                    Ok(()) => {
                        self.applied.insert(d, target);
                        self.last_error = None;
                    }
                    Err(e) => self.last_error = Some(e),
                }
            }
        }

        self.runtime = Runtime {
            active_game_id: hit
                .as_ref()
                .and_then(|h| h.game.as_ref())
                .map(|g| g.id.clone()),
            active_game_name: hit
                .as_ref()
                .and_then(|h| h.game.as_ref())
                .map(|g| g.name.clone()),
            previewing: self.preview_until.is_some(),
            applied_percent: displays.first().and_then(|d| self.applied.get(d)).copied(),
            error: self.last_error.clone(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Fg, WindowInfo};
    use std::sync::{Arc, Mutex};

    struct Log {
        sets: Vec<(DisplayId, i32)>,
        /// Niveau courant de chaque ecran, cote « pilote ».
        levels: HashMap<DisplayId, i32>,
        range: (i32, i32),
        fg: Option<Fg>,
        fail: bool,
    }

    impl Default for Log {
        fn default() -> Self {
            Self {
                sets: Vec::new(),
                levels: HashMap::new(),
                range: (0, 100),
                fg: None,
                fail: false,
            }
        }
    }

    struct FakeDvc(Arc<Mutex<Log>>);
    struct FakeSys(Arc<Mutex<Log>>);

    impl Dvc for FakeDvc {
        fn info(&self) -> GpuInfo {
            let (lo, hi) = self.0.lock().unwrap().range;
            GpuInfo {
                available: true,
                displays: 2,
                min_percent: lo,
                max_percent: hi,
                ..GpuInfo::default()
            }
        }
        fn displays(&self) -> Vec<DisplayId> {
            vec![1, 2]
        }
        fn display_for_monitor(&self, name: &str) -> Option<DisplayId> {
            match name {
                "\\\\.\\DISPLAY1" => Some(1),
                "\\\\.\\DISPLAY2" => Some(2),
                _ => None,
            }
        }
        fn get_percent(&mut self, d: DisplayId) -> Result<i32, String> {
            Ok(*self.0.lock().unwrap().levels.get(&d).unwrap_or(&57))
        }
        fn set_percent(&mut self, d: DisplayId, p: i32) -> Result<(), String> {
            let mut l = self.0.lock().unwrap();
            if l.fail {
                return Err("boom".into());
            }
            l.sets.push((d, p));
            l.levels.insert(d, p);
            Ok(())
        }
    }

    impl Sys for FakeSys {
        fn foreground(&mut self) -> Option<Fg> {
            self.0.lock().unwrap().fg.clone()
        }
        fn windows(&mut self) -> Vec<WindowInfo> {
            Vec::new()
        }
    }

    fn game(exe: &str, percent: Option<i32>) -> Game {
        Game {
            id: exe.to_lowercase(),
            name: exe.trim_end_matches(".exe").to_string(),
            exe: exe.into(),
            path: None,
            percent,
            enabled: true,
        }
    }

    fn fg(exe: &str, monitor: &str) -> Option<Fg> {
        Some(Fg {
            exe: exe.into(),
            path: None,
            title: "t".into(),
            monitor: Some(monitor.into()),
        })
    }

    fn setup(games: Vec<Game>) -> (Engine, Arc<Mutex<Log>>) {
        setup_with(games, Log::default())
    }

    fn setup_with(games: Vec<Game>, log: Log) -> (Engine, Arc<Mutex<Log>>) {
        let log = Arc::new(Mutex::new(log));
        let settings = Settings {
            desktop: 50,
            game_default: 80,
            games,
            ..Settings::default()
        };
        let e = Engine::new(
            settings,
            Box::new(FakeDvc(log.clone())),
            Box::new(FakeSys(log.clone())),
        );
        (e, log)
    }

    fn take(log: &Arc<Mutex<Log>>) -> Vec<(DisplayId, i32)> {
        std::mem::take(&mut log.lock().unwrap().sets)
    }

    #[test]
    fn applies_desktop_level_once_without_redundant_calls() {
        let (mut e, log) = setup(vec![]);
        let t = Instant::now();
        e.tick(t);
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
        e.tick(t);
        assert!(take(&log).is_empty(), "pas de rappel inutile au pilote");
    }

    #[test]
    fn switches_to_game_level_and_back() {
        let (mut e, log) = setup(vec![game("cs2.exe", Some(90))]);
        let t = Instant::now();
        e.tick(t);
        take(&log);

        log.lock().unwrap().fg = fg("CS2.EXE", "\\\\.\\DISPLAY1"); // casse differente
        let (rt, changed) = e.tick(t);
        assert!(changed);
        assert_eq!(rt.active_game_id.as_deref(), Some("cs2.exe"));
        assert_eq!(take(&log), vec![(1, 90), (2, 90)]);

        log.lock().unwrap().fg = fg("explorer.exe", "\\\\.\\DISPLAY1");
        let (rt, _) = e.tick(t);
        assert_eq!(rt.active_game_id, None);
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
    }

    #[test]
    fn game_without_override_follows_default() {
        let (mut e, log) = setup(vec![game("r5apex.exe", None)]);
        log.lock().unwrap().fg = fg("r5apex.exe", "\\\\.\\DISPLAY1");
        e.tick(Instant::now());
        assert_eq!(take(&log), vec![(1, 80), (2, 80)]);
    }

    #[test]
    fn game_monitor_scope_touches_only_that_display() {
        let (mut e, log) = setup(vec![game("cs2.exe", Some(95))]);
        e.update_settings(
            Settings {
                scope: Scope::GameMonitor,
                games: vec![game("cs2.exe", Some(95))],
                ..Settings::default()
            },
            Instant::now(),
        );
        take(&log);
        log.lock().unwrap().fg = fg("cs2.exe", "\\\\.\\DISPLAY2");
        e.tick(Instant::now());
        assert_eq!(take(&log), vec![(2, 95)]);
    }

    #[test]
    fn unknown_monitor_falls_back_to_all_displays() {
        let (mut e, log) = setup(vec![game("cs2.exe", Some(95))]);
        e.update_settings(
            Settings {
                scope: Scope::GameMonitor,
                games: vec![game("cs2.exe", Some(95))],
                ..Settings::default()
            },
            Instant::now(),
        );
        take(&log);
        log.lock().unwrap().fg = fg("cs2.exe", "\\\\.\\DISPLAY9");
        e.tick(Instant::now());
        assert_eq!(take(&log), vec![(1, 95), (2, 95)]);
    }

    #[test]
    fn paused_ignores_games_and_keeps_desktop_level() {
        let (mut e, log) = setup(vec![game("cs2.exe", Some(90))]);
        let mut s = e.settings().clone();
        s.paused = true;
        e.update_settings(s, Instant::now());
        take(&log);
        log.lock().unwrap().fg = fg("cs2.exe", "\\\\.\\DISPLAY1");
        let (rt, _) = e.tick(Instant::now());
        assert_eq!(rt.active_game_id, None);
        assert!(take(&log).is_empty());
    }

    #[test]
    fn disabled_game_is_ignored() {
        let mut g = game("cs2.exe", Some(90));
        g.enabled = false;
        let (mut e, log) = setup(vec![g]);
        e.tick(Instant::now());
        take(&log);
        log.lock().unwrap().fg = fg("cs2.exe", "\\\\.\\DISPLAY1");
        e.tick(Instant::now());
        assert!(take(&log).is_empty());
    }

    #[test]
    fn preview_applies_default_then_expires() {
        let (mut e, log) = setup(vec![]);
        let t = Instant::now();
        e.tick(t);
        take(&log);
        let rt = e.set_preview(true, t);
        assert!(rt.previewing);
        assert_eq!(take(&log), vec![(1, 80), (2, 80)]);

        let (rt, changed) = e.tick(t + PREVIEW_MAX + Duration::from_millis(1));
        assert!(changed && !rt.previewing);
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
    }

    #[test]
    fn driver_errors_are_reported_and_retried() {
        let (mut e, log) = setup(vec![]);
        log.lock().unwrap().fail = true;
        let (rt, _) = e.tick(Instant::now());
        assert_eq!(rt.error.as_deref(), Some("boom"));
        log.lock().unwrap().fail = false;
        let (rt, _) = e.tick(Instant::now());
        assert_eq!(rt.error, None);
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
    }

    #[test]
    fn save_is_debounced() {
        let (mut e, _log) = setup(vec![]);
        let t = Instant::now();
        assert!(e.take_dirty(t).is_none());
        let s = e.settings().clone();
        e.update_settings(s, t);
        assert!(e.take_dirty(t + Duration::from_millis(100)).is_none());
        assert!(e.take_dirty(t + SAVE_DEBOUNCE).is_some());
        assert!(e.take_dirty(t + SAVE_DEBOUNCE * 3).is_none());
    }

    #[test]
    fn restore_puts_desktop_level_back() {
        let (mut e, log) = setup(vec![game("cs2.exe", Some(90))]);
        log.lock().unwrap().fg = fg("cs2.exe", "\\\\.\\DISPLAY1");
        e.tick(Instant::now());
        take(&log);
        e.restore();
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
    }

    #[test]
    fn restore_writes_even_if_cache_says_desktop() {
        let (mut e, log) = setup(vec![]);
        e.tick(Instant::now());
        take(&log);
        log.lock().unwrap().levels.insert(1, 99); // modifie par un autre programme
        e.restore();
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
    }

    #[test]
    fn restore_disabled_does_nothing() {
        let (mut e, log) = setup(vec![]);
        let mut s = e.settings().clone();
        s.restore_on_exit = false;
        e.update_settings(s, Instant::now());
        take(&log);
        e.restore();
        assert!(take(&log).is_empty());
    }

    #[test]
    fn resync_reapplies_level_reset_by_someone_else() {
        let (mut e, log) = setup(vec![game("cs2.exe", Some(90))]);
        log.lock().unwrap().fg = fg("cs2.exe", "\\\\.\\DISPLAY1");
        let t = Instant::now();
        e.tick(t);
        take(&log);

        // Le jeu remet la vibrance par defaut sur l'ecran 1.
        log.lock().unwrap().levels.insert(1, 50);
        e.tick(t + Duration::from_millis(400));
        assert!(
            take(&log).is_empty(),
            "pas de lecture du pilote avant l'intervalle"
        );

        e.tick(t + RESYNC_EVERY);
        assert_eq!(
            take(&log),
            vec![(1, 90)],
            "seul l'ecran modifie est corrige"
        );

        e.tick(t + RESYNC_EVERY * 2);
        assert!(take(&log).is_empty(), "niveau correct : aucune ecriture");
    }

    #[test]
    fn resync_tolerates_rounding() {
        let (mut e, log) = setup(vec![]);
        let t = Instant::now();
        e.tick(t);
        take(&log);
        log.lock().unwrap().levels.insert(1, 51); // arrondi de l'echelle native
        e.tick(t + RESYNC_EVERY);
        assert!(take(&log).is_empty());
    }

    #[test]
    fn levels_are_clamped_to_driver_range() {
        let log = Log {
            range: (50, 100),
            ..Log::default()
        };
        let mut g = game("cs2.exe", Some(10));
        g.enabled = true;
        let (mut e, log) = setup_with(vec![g], log);
        let mut s = e.settings().clone();
        s.desktop = 0;
        e.update_settings(s, Instant::now());
        assert_eq!(e.settings().desktop, 50);
        assert_eq!(e.settings().games[0].percent, Some(50));
        assert_eq!(take(&log), vec![(1, 50), (2, 50)]);
    }

    #[test]
    fn out_of_range_saved_settings_are_rewritten() {
        let log = Arc::new(Mutex::new(Log {
            range: (50, 100),
            ..Log::default()
        }));
        let settings = Settings {
            desktop: 10,
            ..Settings::default()
        };
        let mut e = Engine::new(
            settings,
            Box::new(FakeDvc(log.clone())),
            Box::new(FakeSys(log)),
        );
        assert_eq!(e.settings().desktop, 50);
        assert!(e.take_dirty(Instant::now() + SAVE_DEBOUNCE).is_some());
    }

    #[test]
    fn first_run_adopts_current_driver_level() {
        let (mut e, _log) = setup(vec![]);
        e.adopt_current_as_desktop();
        assert_eq!(e.settings().desktop, 57);
    }
}
