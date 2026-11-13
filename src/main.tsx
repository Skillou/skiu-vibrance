import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/inter";
import "./styles.css";
import { App } from "./App";
import { isTauri } from "./api";

// Thème provisoire selon Windows, pour éviter un flash blanc avant le chargement des réglages.
document.documentElement.dataset.theme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";

// En application installée, on masque les comportements « page web » (menu contextuel,
// rechargement, impression, recherche...) sauf dans les champs de saisie.
if (isTauri && !import.meta.env.DEV) {
  window.addEventListener("contextmenu", (e) => {
    const t = e.target as HTMLElement | null;
    if (!t?.closest("input, textarea")) e.preventDefault();
  });
  window.addEventListener("keydown", (e) => {
    const k = e.key.toLowerCase();
    if (k === "f5" || k === "f7" || ((e.ctrlKey || e.metaKey) && ["r", "p", "f", "g", "u", "j"].includes(k))) e.preventDefault();
  });
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
