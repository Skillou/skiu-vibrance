import { useCallback, useRef } from "react";

interface Props {
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
  orientation?: "vertical" | "horizontal";
  label: string;
  disabled?: boolean;
  tone?: "accent" | "warm";
}

/** Slider personnalisé (pointeur + clavier), vertical façon console ou horizontal. */
export function Slider({ value, min, max, onChange, orientation = "horizontal", label, disabled, tone = "accent" }: Props) {
  const ref = useRef<HTMLDivElement>(null);
  const vertical = orientation === "vertical";
  const pct = max > min ? Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100)) : 0;

  const fromPointer = useCallback(
    (e: React.PointerEvent) => {
      const r = ref.current!.getBoundingClientRect();
      const t = vertical ? 1 - (e.clientY - r.top) / r.height : (e.clientX - r.left) / r.width;
      const v = Math.round(min + Math.min(1, Math.max(0, t)) * (max - min));
      if (v !== value) onChange(v);
    },
    [vertical, min, max, value, onChange],
  );

  return (
    <div
      ref={ref}
      className={`slider ${vertical ? "v" : "h"} tone-${tone}${disabled ? " disabled" : ""}`}
      role="slider"
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={value}
      aria-orientation={orientation}
      tabIndex={disabled ? -1 : 0}
      onPointerDown={(e) => {
        if (disabled) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        fromPointer(e);
      }}
      onPointerMove={(e) => {
        if (!disabled && e.currentTarget.hasPointerCapture(e.pointerId)) fromPointer(e);
      }}
      onKeyDown={(e) => {
        if (disabled) return;
        const step = e.shiftKey ? 10 : 1;
        if (e.key === "ArrowUp" || e.key === "ArrowRight") onChange(Math.min(max, value + step));
        else if (e.key === "ArrowDown" || e.key === "ArrowLeft") onChange(Math.max(min, value - step));
        else if (e.key === "PageUp") onChange(Math.min(max, value + 10));
        else if (e.key === "PageDown") onChange(Math.max(min, value - 10));
        else if (e.key === "Home") onChange(min);
        else if (e.key === "End") onChange(max);
        else return;
        e.preventDefault();
      }}
    >
      <div className="slider-rail">
        <div className="slider-fill" style={vertical ? { height: `${pct}%` } : { width: `${pct}%` }} />
      </div>
      <div className="slider-thumb" style={vertical ? { bottom: `${pct}%` } : { left: `${pct}%` }} />
    </div>
  );
}
