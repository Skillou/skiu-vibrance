import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppState, Picked, Runtime, Settings, WindowInfo } from "./types";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/* ------------------------------------------------------------------ */
/* Mode démo (navigateur) : permet de tester l'interface sans Windows. */
/* ------------------------------------------------------------------ */
const demo = {
  state: {
    settings: {
      desktop: 55,
      gameDefault: 80,
      paused: false,
      scope: "gameMonitor",
      restoreOnExit: true,
      closeToTray: true,
      theme: "auto",
      games: [
        { id: "1", name: "Counter-Strike 2", exe: "cs2.exe", percent: 100, enabled: true },
        { id: "2", name: "VALORANT", exe: "VALORANT-Win64-Shipping.exe", percent: 90, enabled: true },
        { id: "3", name: "Apex Legends", exe: "r5apex.exe", percent: null, enabled: true },
        { id: "4", name: "Cyberpunk 2077", exe: "Cyberpunk2077.exe", percent: 70, enabled: false },
      ],
    },
    gpu: {
      available: true,
      name: "NVIDIA GeForce RTX 4070",
      api: "NVAPI · plage étendue",
      minPercent: 0,
      maxPercent: 100,
      defaultPercent: 50,
      displays: 2,
      error: null,
    },
    runtime: { previewing: false, appliedPercent: 55, activeGameId: null, activeGameName: null },
    autostart: false,
    version: "1.1.0",
  } as AppState,
};

function demoRuntime(): Runtime {
  const s = demo.state.settings;
  const rt = demo.state.runtime;
  return { ...rt, appliedPercent: rt.previewing ? s.gameDefault : s.desktop };
}

const iconCache = new Map<string, Promise<string | null>>();

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const api = {
  async getState(): Promise<AppState> {
    if (isTauri) return invoke<AppState>("get_state");
    await sleep(60);
    return structuredClone({ ...demo.state, runtime: demoRuntime() });
  },
  async updateSettings(settings: Settings): Promise<Runtime> {
    if (isTauri) return invoke<Runtime>("update_settings", { settings });
    demo.state.settings = structuredClone(settings);
    return demoRuntime();
  },
  async setPreview(on: boolean): Promise<Runtime> {
    if (isTauri) return invoke<Runtime>("set_preview", { on });
    demo.state.runtime.previewing = on;
    return demoRuntime();
  },
  async listWindows(): Promise<WindowInfo[]> {
    if (isTauri) return invoke<WindowInfo[]>("list_windows");
    await sleep(120);
    return [
      { exe: "eldenring.exe", title: "ELDEN RING™", path: null },
      { exe: "RocketLeague.exe", title: "Rocket League (64-bit, DX11)", path: null },
      { exe: "Overwatch.exe", title: "Overwatch", path: null },
      { exe: "chrome.exe", title: "Skiu — Google Chrome", path: null },
    ];
  },
  async pickExe(): Promise<Picked | null> {
    if (isTauri) return invoke<Picked | null>("pick_exe");
    return { name: "Fortnite", exe: "FortniteClient-Win64-Shipping.exe", path: "C:\\Games\\Fortnite" };
  },
  /** Icône d'un .exe en data URL PNG (mise en cache : une seule extraction par chemin). */
  exeIcon(path: string): Promise<string | null> {
    const key = path.toLowerCase();
    let p = iconCache.get(key);
    if (!p) {
      p = isTauri ? invoke<string | null>("exe_icon", { path }).catch(() => null) : Promise.resolve(null);
      iconCache.set(key, p);
    }
    return p;
  },
  async setAutostart(enabled: boolean): Promise<boolean> {
    if (isTauri) return invoke<boolean>("set_autostart", { enabled });
    demo.state.autostart = enabled;
    return enabled;
  },
  async onRuntime(cb: (r: Runtime) => void): Promise<() => void> {
    if (!isTauri) return () => {};
    return listen<Runtime>("runtime", (e) => cb(e.payload));
  },
  async onSettings(cb: (s: Settings) => void): Promise<() => void> {
    if (!isTauri) return () => {};
    return listen<Settings>("settings", (e) => cb(e.payload));
  },
};

export const win = {
  minimize: () => (isTauri ? getCurrentWindow().minimize() : undefined),
  toggleMaximize: () => (isTauri ? getCurrentWindow().toggleMaximize() : undefined),
  close: () => (isTauri ? getCurrentWindow().close() : undefined),
};
