import { Minus, Plus } from "lucide-react";
import type { ReactNode } from "react";
import { Slider } from "./Slider";

interface Props {
  title: string;
  hint: string;
  icon: ReactNode;
  value: number;
  min: number;
  max: number;
  presets: number[];
  onChange: (v: number) => void;
  tone?: "accent" | "warm";
  disabled?: boolean;
  badge?: string | null;
}

export function VFader({ title, hint, icon, value, min, max, presets, onChange, tone = "accent", disabled, badge }: Props) {
  const clamp = (v: number) => Math.min(max, Math.max(min, v));
  // Préréglages dans la plage du pilote, sans doublon (ex. défaut pilote = 60 %), triés.
  const chips = [...new Set(presets.map(clamp))].sort((a, b) => a - b);
  return (
    <section className={`glass fader-card tone-${tone}`}>
      <header className="fader-head">
        <span className={`icon-chip tone-${tone}`}>{icon}</span>
        <div>
          <h3>{title}</h3>
          <p>{hint}</p>
        </div>
        {badge && <span className="live-badge">{badge}</span>}
      </header>

      <div className="fader-body">
        <div className="fader-readout">
          <div className="big-number">
            {value}
            <small>%</small>
          </div>
          <div className="stepper">
            <button className="round" onClick={() => onChange(clamp(value - 1))} aria-label={`Diminuer ${title}`} disabled={disabled || value <= min}>
              <Minus size={16} />
            </button>
            <button className="round" onClick={() => onChange(clamp(value + 1))} aria-label={`Augmenter ${title}`} disabled={disabled || value >= max}>
              <Plus size={16} />
            </button>
          </div>
        </div>
        <Slider value={value} min={min} max={max} onChange={onChange} orientation="vertical" label={title} tone={tone} disabled={disabled} />
      </div>

      <div className="chips">
        {chips.map((p) => (
          <button key={p} className={`chip${p === value ? " on" : ""}`} onClick={() => onChange(p)} disabled={disabled}>
            {p}%
          </button>
        ))}
      </div>
    </section>
  );
}
