import { Gamepad2, Minus, Settings2, SlidersHorizontal, Square, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { api, isTauri, win } from "./api";
import { GamesPage } from "./pages/GamesPage";
import { SettingsPage } from "./pages/SettingsPage";
import { VibrancePage } from "./pages/VibrancePage";
import type { AppState, Runtime, Settings } from "./types";

type Tab = "vibrance" | "games" | "settings";

export function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [runtime, setRuntime] = useState<Runtime | null>(null);
  const [autostart, setAutostartState] = useState(false);
  const [tab, setTab] = useState<Tab>(() => {
    const t = new URLSearchParams(location.search).get("tab");
    return t === "games" || t === "settings" ? t : "vibrance";
  });
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const [fatal, setFatal] = useState<string | null>(null);
  const timer = useRef<number>();
  const toastTimer = useRef<number>();
  const latest = useRef<Settings | null>(null);

  const toast = useCallback((m: string) => {
    setToastMsg(m);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToastMsg(null), 2200);
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
      })
      .catch((e) => setFatal(String(e)));
    const offs: Promise<() => void>[] = [
      api.onRuntime(setRuntime),
      api.onSettings((s) => {
        latest.current = s;
        setSettings(s);
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

  const patch = useCallback((p: Partial<Settings>) => {
    const next = { ...latest.current!, ...p };
    latest.current = next;
    setSettings(next);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      api.updateSettings(latest.current!).then(setRuntime).catch(() => toast("Impossible d'appliquer les réglages"));
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
          {tab === "settings" && <SettingsPage state={state} settings={settings} patch={patch} autostart={autostart} setAutostart={setAutostart} />}
        </main>
      </div>

      <div className={`toast${toastMsg ? " show" : ""}`}>{toastMsg}</div>
    </div>
  );
}
