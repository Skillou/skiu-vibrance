export type Scope = "all" | "gameMonitor";
export type Theme = "auto" | "light" | "dark";

export interface Game {
  id: string;
  name: string;
  exe: string;
  path?: string | null;
  percent?: number | null;
  enabled: boolean;
}

export interface Settings {
  desktop: number;
  gameDefault: number;
  games: Game[];
  paused: boolean;
  scope: Scope;
  restoreOnExit: boolean;
  closeToTray: boolean;
  theme: Theme;
  autoUpdate: boolean;
}

export interface GpuInfo {
  available: boolean;
  name: string;
  api: string;
  minPercent: number;
  maxPercent: number;
  defaultPercent: number;
  displays: number;
  error?: string | null;
}

export interface Runtime {
  activeGameId?: string | null;
  activeGameName?: string | null;
  previewing: boolean;
  appliedPercent?: number | null;
  error?: string | null;
}

export interface WindowInfo {
  exe: string;
  title: string;
  path?: string | null;
}

export interface Picked {
  name: string;
  exe: string;
  path: string;
}

export interface AppState {
  settings: Settings;
  gpu: GpuInfo;
  runtime: Runtime;
  autostart: boolean;
  version: string;
}
