//! Logique pure de Skiu Vibrance : reglages, conversion de niveaux et moteur
//! de bascule automatique. Aucun appel systeme ici : tout passe par les traits
//! `Dvc` (pilote graphique) et `Sys` (fenetres), ce qui permet de tester le
//! moteur sans carte NVIDIA ni Windows.

mod engine;
mod model;
mod store;
mod traits;

pub use engine::{Engine, PREVIEW_MAX, RESYNC_EVERY, SAVE_DEBOUNCE};
pub use model::*;
pub use store::{load_settings, save_settings};
pub use traits::{Dvc, Sys};
