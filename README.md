# Skiu Vibrance

Bascule automatiquement la **vibrance numérique NVIDIA** selon le jeu au premier plan : couleurs
saturées en jeu, niveau normal sur le bureau. Inspiré de vibranceGUI, réécrit de zéro
(Tauri 2 + React + Rust).

## Ce que ça fait
- Niveau **Bureau** et niveau **Jeu par défaut**, plus un niveau personnalisé par jeu (renommable, avec la vraie icône de son `.exe`).
- Détection du jeu au premier plan (toutes les 300 ms), sur l'écran du jeu seulement ou sur tous les écrans.
- **Auto-correction** : toutes les 3 s, le niveau réel du pilote est relu et réappliqué si un jeu
  (plein écran exclusif), un changement de résolution ou le Panneau NVIDIA l'a modifié.
- Niveaux automatiquement bornés à la plage réelle du pilote (0–100 % en DVC étendu, 50–100 % en DVC classique).
- **Aperçu** du niveau jeu pendant 12 s, pause, thème clair / sombre / auto.
- Zone de notification avec infobulle d'état en direct (jeu détecté, niveau appliqué), lancement
  avec Windows, restauration du niveau Bureau à la fermeture.
- Réglages protégés : un fichier corrompu est conservé en `settings.json.bak` au lieu d'être écrasé.
- Dialogue direct avec le pilote NVIDIA (NVAPI) : **aucune DLL tierce**, pas de dépendance 32 bits.

Raccourcis : `Ctrl+1/2/3` pour changer d'onglet, flèches / `PgUp` / `PgDn` / `Maj`+flèches sur les curseurs,
`Échap` pour fermer la fenêtre d'ajout de jeu.

## Prérequis (une seule fois)
1. **Node.js LTS** — https://nodejs.org
2. **Rust** (1.82 ou plus récent) — https://rustup.rs
3. **Visual Studio Build Tools** avec « Développement Desktop en C++ ».
4. WebView2 (déjà présent sur Windows 10/11 à jour).

## Installer
Double-clique **BUILD.bat**. À la fin, l'installateur `.exe` s'ouvre dans
`src-tauri\target\release\bundle\nsis`. `DEV.bat` lance la version de développement.

Sans rien installer : pousse un tag `vX.Y.Z` sur GitHub, le workflow **Release** construit
l'installateur et l'attache à un brouillon de release.

## Développement
```bash
npm install
npm run tauri dev                 # application complète
npm run dev                       # interface seule dans le navigateur (mode démo)
cd src-tauri && cargo test -p skiu-core   # tests de la logique (27 tests ; `cargo test -p skiu-win` sous Windows : 3 de plus)
```
La CI (GitHub Actions) vérifie à chaque push : build de l'interface, `cargo fmt`, tests, clippy et
les types Windows de `skiu-win`.

## Limites connues
- **NVIDIA uniquement.** Sans carte NVIDIA, l'application s'ouvre et l'indique dans le diagnostic.
- Certains jeux en plein écran exclusif peuvent ignorer la vibrance du bureau ; le mode fenêtré sans
  bordure est le plus fiable.

## Structure
- `src/` interface React · `src-tauri/src` colle Tauri (commandes, zone de notification, surveillance)
- `src-tauri/crates/skiu-core` logique pure et testée (moteur, réglages, stockage)
- `src-tauri/crates/skiu-win` accès NVAPI / Windows

Projet indépendant, non affilié à vibranceGUI ni à NVIDIA.
