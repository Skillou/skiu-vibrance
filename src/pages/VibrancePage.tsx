import { Eye, Gamepad2, Monitor, PauseCircle, Sparkles, TriangleAlert } from "lucide-react";
import { AppIcon } from "../components/AppIcon";
import { VFader } from "../components/VFader";
import type { GpuInfo, Runtime, Settings } from "../types";

interface Props {
  settings: Settings;
  gpu: GpuInfo;
  runtime: Runtime;
  patch: (p: Partial<Settings>) => void;
  preview: (on: boolean) => void;
}

export function VibrancePage({ settings, gpu, runtime, patch, preview }: Props) {
  const inGame = !!runtime.activeGameName && !settings.paused;
  const activeGame = settings.games.find((g) => g.id === runtime.activeGameId);
  const status = settings.paused
    ? { icon: <PauseCircle size={20} />, title: "Bascule en pause", sub: "Le niveau Bureau reste appliqué partout." }
    : runtime.previewing
      ? { icon: <Eye size={20} />, title: "Aperçu en cours", sub: "Le niveau Jeu est appliqué quelques secondes." }
      : inGame
        ? { icon: <Gamepad2 size={20} />, title: runtime.activeGameName!, sub: "Jeu détecté — vibrance de jeu appliquée." }
        : { icon: <Monitor size={20} />, title: "Bureau", sub: "Aucun jeu actif — vibrance de bureau." };

  const min = gpu.minPercent;
  const max = gpu.maxPercent;

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">Vibrance numérique</div>
          <h1>Des couleurs qui s'adaptent à ce que tu joues.</h1>
        </div>
      </div>

      {!gpu.available && (
        <div className="notice warn">
          <TriangleAlert size={18} />
          <div>
            <strong>Carte NVIDIA non détectée</strong>
            <p>{gpu.error ?? "Aucun pilote NVIDIA compatible n'a répondu."}</p>
          </div>
        </div>
      )}

      {gpu.available && runtime.error && (
        <div className="notice warn" role="alert">
          <TriangleAlert size={18} />
          <div>
            <strong>Le pilote a refusé le changement de vibrance</strong>
            <p>{runtime.error} — nouvelle tentative automatique.</p>
          </div>
        </div>
      )}

      <section className={`glass hero${inGame || runtime.previewing ? " live" : ""}`}>
        {inGame && !runtime.previewing && activeGame?.path ? (
          <AppIcon path={activeGame.path} name={activeGame.name} />
        ) : (
          <span className="hero-icon">{status.icon}</span>
        )}
        <div className="hero-text">
          <div className="eyebrow">État actuel</div>
          <h2>{status.title}</h2>
          <p>{status.sub}</p>
        </div>
        <div className="hero-value">
          <div className="big-number sm">
            {runtime.appliedPercent ?? "–"}
            {runtime.appliedPercent != null && <small>%</small>}
          </div>
          <span>appliqué</span>
        </div>
        <button
          className={`btn ${runtime.previewing ? "primary" : ""}`}
          disabled={!gpu.available || settings.paused}
          onClick={() => preview(!runtime.previewing)}
        >
          <Sparkles size={16} />
          {runtime.previewing ? "Arrêter l'aperçu" : "Aperçu du niveau jeu"}
        </button>
      </section>

      <div className="fader-grid">
        <VFader
          title="Bureau"
          hint="Quand aucun jeu n'est au premier plan"
          icon={<Monitor size={18} />}
          value={settings.desktop}
          min={min}
          max={max}
          presets={[gpu.defaultPercent, 60, 70]}
          onChange={(v) => patch({ desktop: v })}
          disabled={!gpu.available}
          badge={!inGame && !runtime.previewing && !settings.paused ? "Actif" : null}
        />
        <VFader
          title="Jeu par défaut"
          hint="Pour tout jeu sans réglage personnalisé"
          icon={<Gamepad2 size={18} />}
          tone="warm"
          value={settings.gameDefault}
          min={min}
          max={max}
          presets={[70, 80, 90, 100]}
          onChange={(v) => patch({ gameDefault: v })}
          disabled={!gpu.available}
          badge={inGame || runtime.previewing ? "Actif" : null}
        />
      </div>
    </div>
  );
}
