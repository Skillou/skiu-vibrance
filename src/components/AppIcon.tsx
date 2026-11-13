import { useEffect, useState } from "react";
import { api } from "../api";

/** Icône réelle de l'exécutable, ou initiale du nom en secours. */
export function AppIcon({ path, name, small }: { path?: string | null; name: string; small?: boolean }) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    setSrc(null);
    if (!path) return;
    let alive = true;
    api.exeIcon(path).then((s) => alive && setSrc(s));
    return () => {
      alive = false;
    };
  }, [path]);

  const cls = `game-logo${small ? " sm" : ""}`;
  if (src) {
    return (
      <span className={`${cls} has-icon`}>
        <img src={src} alt="" draggable={false} />
      </span>
    );
  }
  return <span className={cls}>{name.trim().slice(0, 1).toUpperCase() || "?"}</span>;
}
