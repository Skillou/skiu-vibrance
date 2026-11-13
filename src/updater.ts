import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "./api";

export type UpdateStatus = "idle" | "checking" | "upToDate" | "available" | "downloading" | "installing" | "error";

export interface UpdateState {
  status: UpdateStatus;
  /** Nouvelle version disponible. */
  version?: string;
  date?: string;
  notes?: string;
  /** Progression du téléchargement, 0 à 1 (null si taille inconnue). */
  progress: number | null;
  error?: string;
  lastCheck?: Date;
}

/** Nouvelle vérification en arrière-plan tant que l'application tourne. */
const RECHECK_EVERY_MS = 6 * 60 * 60 * 1000;
/** Délai avant la première vérification, pour ne pas ralentir le démarrage. */
const FIRST_CHECK_DELAY_MS = 4000;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function errorText(e: unknown): string {
  const s = String(e instanceof Error ? e.message : e);
  if (/network|dns|connect|timed? ?out|request/i.test(s)) return "Impossible de joindre le serveur de mises à jour (connexion Internet ?).";
  if (/signature/i.test(s)) return "Signature de la mise à jour invalide : installation refusée par sécurité.";
  if (/404|not found|latest\.json/i.test(s)) return "Aucune version publiée pour l'instant.";
  return s;
}

export function useUpdater(auto: boolean) {
  const [state, setState] = useState<UpdateState>({ status: "idle", progress: null });
  const pending = useRef<Update | null>(null);
  const busy = useRef(false);

  /** `silent` : en cas d'erreur ou si à jour, on ne dérange pas l'utilisateur. */
  const checkNow = useCallback(async (silent = false) => {
    if (busy.current) return;
    busy.current = true;
    if (!silent) setState((s) => ({ ...s, status: "checking", error: undefined }));
    try {
      if (!isTauri) {
        await sleep(700);
        setState({ status: "available", version: "1.3.0", date: new Date().toISOString(), notes: "Exemple de notes de version (mode démo).", progress: null, lastCheck: new Date() });
        return;
      }
      const update = await check({ timeout: 20000 });
      pending.current = update;
      if (update) {
        setState({ status: "available", version: update.version, date: update.date, notes: update.body, progress: null, lastCheck: new Date() });
      } else {
        setState((s) => ({ status: silent && s.status === "idle" ? "idle" : "upToDate", progress: null, lastCheck: new Date() }));
      }
    } catch (e) {
      setState((s) => (silent ? { ...s, lastCheck: new Date() } : { status: "error", error: errorText(e), progress: null, lastCheck: new Date() }));
    } finally {
      busy.current = false;
    }
  }, []);

  const install = useCallback(async () => {
    if (busy.current) return;
    busy.current = true;
    setState((s) => ({ ...s, status: "downloading", progress: 0, error: undefined }));
    try {
      if (!isTauri) {
        for (let i = 1; i <= 20; i++) {
          await sleep(80);
          setState((s) => ({ ...s, progress: i / 20 }));
        }
        setState((s) => ({ ...s, status: "installing" }));
        return;
      }
      const update = pending.current ?? (await check({ timeout: 20000 }));
      if (!update) {
        setState({ status: "upToDate", progress: null, lastCheck: new Date() });
        return;
      }
      let total = 0;
      let done = 0;
      await update.downloadAndInstall((ev) => {
        if (ev.event === "Started") {
          total = ev.data.contentLength ?? 0;
          setState((s) => ({ ...s, progress: total ? 0 : null }));
        } else if (ev.event === "Progress") {
          done += ev.data.chunkLength;
          if (total) setState((s) => ({ ...s, progress: Math.min(1, done / total) }));
        } else if (ev.event === "Finished") {
          setState((s) => ({ ...s, status: "installing", progress: 1 }));
        }
      });
      // Sous Windows l'installateur ferme l'application lui-même ; ailleurs on relance.
      await relaunch();
    } catch (e) {
      setState((s) => ({ ...s, status: "error", error: errorText(e), progress: null }));
    } finally {
      busy.current = false;
    }
  }, []);

  useEffect(() => {
    if (!auto) return;
    const first = window.setTimeout(() => checkNow(true), FIRST_CHECK_DELAY_MS);
    const again = window.setInterval(() => checkNow(true), RECHECK_EVERY_MS);
    return () => {
      window.clearTimeout(first);
      window.clearInterval(again);
    };
  }, [auto, checkNow]);

  return { state, checkNow, install };
}

export type Updater = ReturnType<typeof useUpdater>;
