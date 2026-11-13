import { FolderOpen, Gamepad2, Pencil, Plus, Search, Trash2, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { api } from "../api";
import { Slider } from "../components/Slider";
import { Switch } from "../components/Switch";
import type { Game, GpuInfo, Runtime, Settings, WindowInfo } from "../types";

interface Props {
  settings: Settings;
  gpu: GpuInfo;
  runtime: Runtime;
  patch: (p: Partial<Settings>) => void;
  toast: (m: string) => void;
}

const uid = () => (crypto.randomUUID ? crypto.randomUUID() : String(Date.now() + Math.random()));

/** Nom lisible pour une fenêtre : son titre s'il est court, sinon le nom du fichier. */
const displayName = (w: WindowInfo) => {
  const t = w.title.trim();
  return t.length > 0 && t.length < 40 ? t : w.exe.replace(/\.exe$/i, "");
};

function GameName({ name, onRename }: { name: string; onRename: (n: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(name);

  const commit = () => {
    setEditing(false);
    const n = draft.trim();
    if (n && n !== name) onRename(n);
  };

  if (editing) {
    return (
      <input
        className="name-input"
        autoFocus
        value={draft}
        maxLength={60}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          else if (e.key === "Escape") {
            e.stopPropagation();
            setEditing(false);
          }
        }}
        aria-label="Nom du jeu"
      />
    );
  }
  return (
    <button
      className="name-btn"
      title="Renommer"
      onClick={() => {
        setDraft(name);
        setEditing(true);
      }}
    >
      <h3>{name}</h3>
      <Pencil size={12} />
    </button>
  );
}

export function GamesPage({ settings, gpu, runtime, patch, toast }: Props) {
  const [adding, setAdding] = useState(false);
  const [wins, setWins] = useState<WindowInfo[] | null>(null);
  const [query, setQuery] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!adding) return;
    setWins(null);
    setQuery("");
    api.listWindows().then(setWins).catch(() => setWins([]));
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setAdding(false);
    window.addEventListener("keydown", onKey);
    requestAnimationFrame(() => searchRef.current?.focus());
    return () => window.removeEventListener("keydown", onKey);
  }, [adding]);

  const update = (id: string, p: Partial<Game>) => patch({ games: settings.games.map((g) => (g.id === id ? { ...g, ...p } : g)) });

  const remove = (g: Game) => {
    patch({ games: settings.games.filter((x) => x.id !== g.id) });
    toast(`${g.name} retiré`);
  };

  const add = (name: string, exe: string, path?: string | null) => {
    if (settings.games.some((g) => g.exe.toLowerCase() === exe.toLowerCase())) {
      toast("Ce jeu est déjà dans la liste");
      return;
    }
    patch({ games: [...settings.games, { id: uid(), name, exe, path: path ?? null, percent: null, enabled: true }] });
    setAdding(false);
    toast(`${name} ajouté`);
  };

  const browse = async () => {
    try {
      const p = await api.pickExe();
      if (p) add(p.name, p.exe, p.path);
    } catch {
      toast("Impossible d'ouvrir le sélecteur de fichiers");
    }
  };

  const known = new Set(settings.games.map((g) => g.exe.toLowerCase()));
  const q = query.trim().toLowerCase();
  const candidates = (wins ?? []).filter(
    (w) => !known.has(w.exe.toLowerCase()) && (!q || w.exe.toLowerCase().includes(q) || w.title.toLowerCase().includes(q)),
  );

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <div className="eyebrow">Bibliothèque · {settings.games.length} jeu{settings.games.length > 1 ? "x" : ""}</div>
          <h1>Tes jeux</h1>
        </div>
        <button className="btn primary" onClick={() => setAdding(true)}>
          <Plus size={16} /> Ajouter un jeu
        </button>
      </div>

      {settings.games.length === 0 ? (
        <div className="glass empty">
          <Gamepad2 size={34} />
          <h3>Aucun jeu pour l'instant</h3>
          <p>Ajoute un jeu pour lui donner son propre niveau de vibrance, ou laisse le niveau par défaut s'appliquer.</p>
          <button className="btn primary" onClick={() => setAdding(true)}>
            <Plus size={16} /> Ajouter un jeu
          </button>
        </div>
      ) : (
        <div className="game-list">
          {settings.games.map((g) => {
            const active = runtime.activeGameId === g.id && !settings.paused;
            const custom = g.percent != null;
            const value = g.percent ?? settings.gameDefault;
            return (
              <article key={g.id} className={`glass game${active ? " active" : ""}${g.enabled ? "" : " off"}`}>
                <div className="game-logo">{g.name.slice(0, 1).toUpperCase()}</div>
                <div className="game-main">
                  <div className="game-title">
                    <GameName name={g.name} onRename={(name) => update(g.id, { name })} />
                    {active && <span className="live-badge">En jeu</span>}
                  </div>
                  <code title={g.path ?? g.exe}>{g.exe}</code>
                </div>
                <div className="game-slider">
                  <Slider
                    value={value}
                    min={gpu.minPercent}
                    max={gpu.maxPercent}
                    onChange={(v) => update(g.id, { percent: v })}
                    label={`Vibrance ${g.name}`}
                    tone="warm"
                    disabled={!g.enabled}
                  />
                  <div className="game-val">
                    <strong>{value}%</strong>
                    {custom ? (
                      <button className="link" onClick={() => update(g.id, { percent: null })}>
                        Réinitialiser
                      </button>
                    ) : (
                      <span>par défaut</span>
                    )}
                  </div>
                </div>
                <Switch on={g.enabled} onChange={(v) => update(g.id, { enabled: v })} label={`Activer ${g.name}`} />
                <button className="round ghost" onClick={() => remove(g)} aria-label={`Supprimer ${g.name}`} title="Supprimer">
                  <Trash2 size={16} />
                </button>
              </article>
            );
          })}
        </div>
      )}

      {adding && (
        <div className="scrim" onMouseDown={() => setAdding(false)}>
          <div className="glass sheet" role="dialog" aria-modal="true" aria-label="Ajouter un jeu" onMouseDown={(e) => e.stopPropagation()}>
            <header>
              <div>
                <h3>Ajouter un jeu</h3>
                <p>Lance ton jeu, puis choisis-le dans la liste des fenêtres ouvertes.</p>
              </div>
              <button className="round ghost" onClick={() => setAdding(false)} aria-label="Fermer">
                <X size={16} />
              </button>
            </header>
            <label className="search">
              <Search size={15} />
              <input
                ref={searchRef}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Filtrer par nom ou par .exe"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && candidates.length === 1) add(displayName(candidates[0]), candidates[0].exe, candidates[0].path);
                }}
              />
            </label>
            <div className="sheet-list">
              {wins === null && <div className="sheet-empty">Recherche des fenêtres…</div>}
              {wins !== null && candidates.length === 0 && (
                <div className="sheet-empty">{q ? "Aucune fenêtre ne correspond." : "Aucune fenêtre disponible. Utilise « Parcourir »."}</div>
              )}
              {candidates.map((w) => (
                <button key={w.exe} className="sheet-row" onClick={() => add(displayName(w), w.exe, w.path)}>
                  <span className="game-logo sm">{w.exe.slice(0, 1).toUpperCase()}</span>
                  <span className="sheet-row-text">
                    <strong>{w.title || w.exe}</strong>
                    <code>{w.exe}</code>
                  </span>
                  <Plus size={16} />
                </button>
              ))}
            </div>
            <footer>
              <button className="btn" onClick={browse}>
                <FolderOpen size={16} /> Parcourir un .exe…
              </button>
            </footer>
          </div>
        </div>
      )}
    </div>
  );
}
