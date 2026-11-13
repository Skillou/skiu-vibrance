import { CheckCircle2, Cpu, Download, Layers, Loader2, Monitor, Moon, RefreshCw, Sun, SunMoon, TriangleAlert, Tv } from "lucide-react";
import { Segmented } from "../components/Segmented";
import { Switch } from "../components/Switch";
import type { AppState, Settings } from "../types";
import type { Updater } from "../updater";

interface Props {
  state: AppState;
  settings: Settings;
  patch: (p: Partial<Settings>) => void;
  autostart: boolean;
  setAutostart: (v: boolean) => void;
  updater: Updater;
}

function Row({ title, desc, children }: { title: string; desc: string; children: React.ReactNode }) {
  return (
    <div className="row">
      <div>
        <strong>{title}</strong>
        <p>{desc}</p>
      </div>
      <div className="row-ctl">{children}</div>
    </div>
  );
}

function UpdateSection({ version, settings, patch, updater }: { version: string; settings: Settings; patch: Props["patch"]; updater: Updater }) {
  const { state: u, checkNow, install } = updater;
  const busy = u.status === "checking" || u.status === "downloading" || u.status === "installing";
  const pct = u.progress != null ? Math.round(u.progress * 100) : null;
  const since = u.lastCheck ? u.lastCheck.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }) : null;

  let line: React.ReactNode;
  switch (u.status) {
    case "checking":
      line = <><Loader2 size={14} className="spin" /> Recherche en cours…</>;
      break;
    case "upToDate":
      line = <><CheckCircle2 size={14} className="ok" /> Tu as la dernière version{since ? ` (vérifié à ${since})` : ""}.</>;
      break;
    case "available":
      line = <><Download size={14} className="accent" /> Version {u.version} disponible.</>;
      break;
    case "downloading":
      line = <><Loader2 size={14} className="spin" /> Téléchargement{pct != null ? ` ${pct} %` : "…"}</>;
      break;
    case "installing":
      line = <><Loader2 size={14} className="spin" /> Installation, l'application va redémarrer…</>;
      break;
    case "error":
      line = <><TriangleAlert size={14} className="bad" /> {u.error}</>;
      break;
    default:
      line = <>Version installée : {version}</>;
  }

  return (
    <section className="glass group">
      <div className="row">
        <div>
          <strong>Mises à jour</strong>
          <p className="update-line">{line}</p>
        </div>
        <div className="row-ctl">
          {u.status === "available" ? (
            <button className="btn primary" onClick={install}>
              <Download size={16} /> Installer et redémarrer
            </button>
          ) : (
            <button className="btn" onClick={() => checkNow(false)} disabled={busy}>
              <RefreshCw size={16} className={u.status === "checking" ? "spin" : ""} /> Rechercher
            </button>
          )}
        </div>
      </div>
      {(u.status === "downloading" || u.status === "installing") && (
        <div className="progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined}>
          <i style={{ width: `${pct ?? 100}%` }} className={pct == null ? "indeterminate" : ""} />
        </div>
      )}
      {u.status === "available" && u.notes && (
        <div className="release-notes">
          <span className="eyebrow">Nouveautés {u.version}</span>
          <p>{u.notes}</p>
        </div>
      )}
      <Row title="Vérifier automatiquement" desc="Au démarrage puis toutes les 6 heures. Rien n'est installé sans ton accord.">
        <Switch on={settings.autoUpdate} onChange={(autoUpdate) => patch({ autoUpdate })} label="Vérifier automatiquement les mises à jour" />
      </Row>
    </section>
  );
}

export function SettingsPage({ state, settings, patch, autostart, setAutostart, updater }: Props) {
  const g = state.gpu;
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">Préférences</div>
          <h1>Réglages</h1>
        </div>
      </div>

      <section className="glass group">
        <Row title="Apparence" desc="Clair, sombre, ou selon Windows.">
          <Segmented
            value={settings.theme}
            onChange={(theme) => patch({ theme })}
            options={[
              { value: "auto", label: "Auto", icon: <SunMoon size={14} /> },
              { value: "light", label: "Clair", icon: <Sun size={14} /> },
              { value: "dark", label: "Sombre", icon: <Moon size={14} /> },
            ]}
          />
        </Row>
        <Row title="Écrans concernés" desc="Appliquer la vibrance de jeu à tous les écrans ou seulement à celui du jeu.">
          <Segmented
            value={settings.scope}
            onChange={(scope) => patch({ scope })}
            options={[
              { value: "gameMonitor", label: "Écran du jeu", icon: <Monitor size={14} /> },
              { value: "all", label: "Tous", icon: <Tv size={14} /> },
            ]}
          />
        </Row>
      </section>

      <section className="glass group">
        <Row title="Lancer avec Windows" desc="Démarre discrètement dans la zone de notification.">
          <Switch on={autostart} onChange={setAutostart} label="Lancer avec Windows" />
        </Row>
        <Row title="Réduire dans la zone de notification" desc="Fermer la fenêtre garde la bascule active en arrière-plan.">
          <Switch on={settings.closeToTray} onChange={(closeToTray) => patch({ closeToTray })} label="Réduire à la fermeture" />
        </Row>
        <Row title="Restaurer à la fermeture" desc="Remet la vibrance de bureau quand tu quittes l'application.">
          <Switch on={settings.restoreOnExit} onChange={(restoreOnExit) => patch({ restoreOnExit })} label="Restaurer à la fermeture" />
        </Row>
        <Row title="Mettre en pause" desc="Désactive temporairement la bascule automatique.">
          <Switch on={settings.paused} onChange={(paused) => patch({ paused })} label="Pause" />
        </Row>
      </section>

      <UpdateSection version={state.version} settings={settings} patch={patch} updater={updater} />

      <section className="glass group diag">
        <div className="diag-head">
          <Cpu size={18} />
          <strong>Diagnostic</strong>
          <span className={`dot ${g.available ? "ok" : "bad"}`} />
          <span className="diag-state">{g.available ? "Pilote NVIDIA joignable" : "Indisponible"}</span>
        </div>
        <dl>
          <div><dt>Carte</dt><dd>{g.available ? g.name : "—"}</dd></div>
          <div><dt>Interface</dt><dd>{g.available ? g.api : "—"}</dd></div>
          <div><dt>Plage</dt><dd>{g.available ? `${g.minPercent}–${g.maxPercent} %` : "—"}</dd></div>
          <div><dt>Écrans</dt><dd>{g.available ? g.displays : "—"}</dd></div>
          <div><dt>Version</dt><dd>{state.version}</dd></div>
        </dl>
        {g.error && <p className="diag-err">{g.error}</p>}
        <p className="diag-note">
          <Layers size={14} /> Ne fonctionne qu'avec les cartes NVIDIA. Skiu Vibrance utilise l'interface officielle du pilote, sans DLL tierce.
        </p>
      </section>
    </div>
  );
}
