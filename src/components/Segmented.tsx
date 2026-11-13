import type { ReactNode } from "react";

interface Opt<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: Opt<T>[]; onChange: (v: T) => void }) {
  const idx = Math.max(0, options.findIndex((o) => o.value === value));
  return (
    <div className="segmented" style={{ ["--n" as string]: options.length, ["--i" as string]: idx }}>
      <i className="seg-thumb" />
      {options.map((o) => (
        <button key={o.value} className={o.value === value ? "on" : ""} onClick={() => onChange(o.value)}>
          {o.icon}
          {o.label}
        </button>
      ))}
    </div>
  );
}
