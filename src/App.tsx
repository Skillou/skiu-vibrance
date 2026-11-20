import { ArrowDownCircle, Gamepad2, Minus, Settings2, SlidersHorizontal, Square, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { api, isTauri, win } from "./api";
import { GamesPage } from "./pages/GamesPage";
import { SettingsPage } from "./pages/SettingsPage";
import { VibrancePage } from "./pages/VibrancePage";
import type { AppState, Patch, Runtime, Settings, Toast, ToastAction } from "./types";
import { useUpdater } from "./updater";

type Tab = "vibrance" | "games" | "settings";

/** Fusion à trois voies : les champs modifiés localement depuis `base` l'emportent sur `theirs`. */
function mergeSettings(base: Settings, ours: Settings, theirs: Settings): Settings {
  const out = { ...theirs } as Record<keyof Settings, unknown>;
  for (const k of Object.keys(ours) as (keyof Settings)[]) {
    if (ours[k] !== base[k]) out[k] = ours[k];
  }
  return out as Settings;
}

export function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [runtime, setRuntime] = useState<Runtime | null>(null);
  const [autostart, setAutostartState] = useState(false);
  const [tab, setTab] = useState<Tab>(() => {
    const t = new URLSearchParams(location.search).get("tab");
    return t === "games" || t === "settings" ? t : "vibrance";
  });
  // Le contenu reste en place pendant le fondu de sortie ; `toastOn` pilote l'affichage.
  const [toastData, setToastData] = useState<{ message: string; action?: ToastAction }>({ message: "" });
  const [toastOn, setToastOn] = useState(false);
  const [fatal, setFatal] = useState<string | null>(null);
  const timer = useRef<number>();
  const toastTimer = useRef<number>();
  const latest = useRef<Settings | null>(null);
  /** Derniers réglages connus du moteur (envoyés ou reçus) : base de la fusion. */
  const synced = useRef<Settings | null>(null);
  const updater = useUpdater(!!settings?.autoUpdate);

  const toast = useCallback<Toast>((message, action) => {
    setToastData({ message, action });
    setToastOn(true);
    window.clearTimeout(toastTimer.current);
    // Un message avec action (« Annuler ») reste plus longtemps à l'écran.
    toastTimer.current = window.setTimeout(() => setToastOn(false), action ? 6000 : 2200);
  }, []);

  useEffect(() => {
    api
      .getState()
      .then((s) => {
        setState(s);
        setSettings(s.settings);
        setRuntime(s.runtime);
        setAutostartState(s.autostart);
        latest.current = s.settings;
        synced.current = s.settings;
      })
      .catch((e) => setFatal(String(e)));
    const offs: Promise<() => void>[] = [
      api.onRuntime(setRuntime),
      // Réglages modifiés par le moteur (pause depuis la zone de notification, chemin appris).
      // Une modification de l'interface pas encore envoyée n'est pas écrasée : seuls les
      // champs qu'elle n'a pas touchés prennent la valeur du moteur.
      api.onSettings((theirs) => {
        const ours = latest.current;
        const base = synced.current;
        synced.current = theirs;
        const next = timer.current !== undefined && ours && base ? mergeSettings(base, ours, theirs) : theirs;
        latest.current = next;
        setSettings(next);
      }),
      api.onSaveError((error) => {
        if (error) toast("Impossible d'enregistrer les réglages, nouvelle tentative dans quelques secondes");
        else toast("Réglages enregistrés");
      }),
    ];
    return () => {
      offs.forEach((p) => p.then((f) => f()));
    };
  }, []);

  // Thème : auto suit Windows
  useEffect(() => {
    if (!settings) return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const dark = settings.theme === "dark" || (settings.theme === "auto" && mq.matches);
      document.documentElement.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings?.theme]);

  const patch = useCallback<Patch>((p) => {
    const current = latest.current!;
    const next = { ...current, ...(typeof p === "function" ? p(current) : p) };
    latest.current = next;
    setSettings(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = undefined;
      const sent = latest.current!;
      synced.current = sent;
      api.updateSettings(sent).then(setRuntime).catch(() => toast("Impossible d'appliquer les réglages"));
    }, 50);
  }, [toast]);

  const preview = useCallback(
    (on: boolean) => {
      api.setPreview(on).then(setRuntime).catch(() => toast("Aperçu impossible"));
    },
    [toast],
  );

  const setAutostart = useCallback(
    (v: boolean) => {
      setAutostartState(v);
      api
        .setAutostart(v)
        .then(setAutostartState)
        .catch(() => {
          setAutostartState(!v);
          toast("Impossible de modifier le démarrage automatique");
        });
    },
    [toast],
  );

  // Menu de la zone de notification : « Rechercher les mises à jour… »
  const checkUpdate = updater.checkNow;
  useEffect(() => {
    const off = api.onCheckUpdate(() => {
      setTab("settings");
      checkUpdate(false);
    });
    return () => {
      off.then((f) => f());
    };
  }, [checkUpdate]);

  // Ctrl+1/2/3 : navigation rapide entre les onglets.
  useEffect(() => {
    const tabs: Tab[] = ["vibrance", "games", "settings"];
    const onKey = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.altKey || e.shiftKey) return;
      const i = Number(e.key) - 1;
      if (i >= 0 && i < tabs.length) {
        e.preventDefault();
        setTab(tabs[i]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (fatal) return <div className="boot">Erreur de démarrage : {fatal}</div>;
  if (!state || !settings || !runtime) return <div className="boot" />;

  const nav: { id: Tab; label: string; icon: JSX.Element }[] = [
    { id: "vibrance", label: "Vibrance", icon: <SlidersHorizontal size={18} /> },
    { id: "games", label: "Jeux", icon: <Gamepad2 size={18} /> },
    { id: "settings", label: "Réglages", icon: <Settings2 size={18} /> },
  ];

  const inGame = !!runtime.activeGameName && !settings.paused;

  return (
    <div className="shell">
      <div className="bg-orb a" />
      <div className="bg-orb b" />

      <header className="titlebar" data-tauri-drag-region>
        <div className="brand" data-tauri-drag-region>
          <img src="/icon.svg" alt="" width={22} height={22} />
          <span>Skiu Vibrance</span>
        </div>
        {isTauri && (
          <div className="wincontrols">
            <button onClick={() => win.minimize()} aria-label="Réduire"><Minus size={15} /></button>
            <button onClick={() => win.toggleMaximize()} aria-label="Agrandir"><Square size={12} /></button>
            <button className="close" onClick={() => win.close()} aria-label="Fermer"><X size={15} /></button>
          </div>
        )}
      </header>

      <div className="body">
        <aside className="glass sidebar">
          <nav>
            {nav.map((n, i) => (
              <button key={n.id} className={tab === n.id ? "on" : ""} onClick={() => setTab(n.id)} title={`Ctrl+${i + 1}`} aria-current={tab === n.id ? "page" : undefined}>
                {n.icon}
                {n.label}
              </button>
            ))}
          </nav>
          {(updater.state.status === "available" || updater.state.status === "downloading" || updater.state.status === "installing") && (
            <button className="update-pill" onClick={() => setTab("settings")}>
              <ArrowDownCircle size={16} />
              <span>
                <strong>Mise à jour {updater.state.version}</strong>
                <small>{updater.state.status === "available" ? "Cliquer pour installer" : "Installation…"}</small>
              </span>
            </button>
          )}
          <div className="side-status">
            <span className={`dot ${state.gpu.available ? (settings.paused ? "warn" : "ok") : "bad"}`} />
            <div>
              <strong>{!state.gpu.available ? "GPU indisponible" : settings.paused ? "En pause" : inGame ? "En jeu" : "Surveillance active"}</strong>
              <span>{state.gpu.available ? state.gpu.name.replace(/^NVIDIA\s+/i, "") : "NVIDIA requis"}</span>
            </div>
          </div>
        </aside>

        <main>
          {tab === "vibrance" && <VibrancePage settings={settings} gpu={state.gpu} runtime={runtime} patch={patch} preview={preview} />}
          {tab === "games" && <GamesPage settings={settings} gpu={state.gpu} runtime={runtime} patch={patch} toast={toast} />}
          {tab === "settings" && <SettingsPage state={state} settings={settings} patch={patch} autostart={autostart} setAutostart={setAutostart} updater={updater} />}
        </main>
      </div>

      <div className={`toast${toastOn ? " show" : ""}`} role="status" aria-live="polite">
        {toastData.message}
        {toastData.action && (
          <button
            tabIndex={toastOn ? 0 : -1}
            onClick={() => {
              toastData.action!.run();
              window.clearTimeout(toastTimer.current);
              setToastOn(false);
            }}
          >
            {toastData.action.label}
          </button>
        )}
      </div>
    </div>
  );
}
