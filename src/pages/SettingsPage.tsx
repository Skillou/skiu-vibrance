import { Cpu, Layers, Monitor, Moon, Power, Sun, SunMoon, Tv } from "lucide-react";
import { Segmented } from "../components/Segmented";
import { Switch } from "../components/Switch";
import type { AppState, Settings } from "../types";

interface Props {
  state: AppState;
  settings: Settings;
  patch: (p: Partial<Settings>) => void;
  autostart: boolean;
  setAutostart: (v: boolean) => void;
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

export function SettingsPage({ state, settings, patch, autostart, setAutostart }: Props) {
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
