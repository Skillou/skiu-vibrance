use serde::{Deserialize, Serialize};

/// Identifiant d'un ecran cote pilote (valeur opaque du handle NVAPI).
pub type DisplayId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// La vibrance de jeu s'applique a tous les ecrans.
    #[default]
    All,
    /// Seulement a l'ecran ou se trouve la fenetre du jeu.
    GameMonitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    /// Nom du fichier, ex. `cs2.exe` (comparaison insensible a la casse).
    pub exe: String,
    #[serde(default)]
    pub path: Option<String>,
    /// `None` = suit le niveau « en jeu » par defaut.
    #[serde(default)]
    pub percent: Option<i32>,
    #[serde(default = "yes")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Niveau au bureau, en % (echelle du Panneau NVIDIA).
    pub desktop: i32,
    /// Niveau par defaut en jeu, en %.
    pub game_default: i32,
    pub games: Vec<Game>,
    pub paused: bool,
    pub scope: Scope,
    pub restore_on_exit: bool,
    pub close_to_tray: bool,
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            desktop: 50,
            game_default: 80,
            games: Vec::new(),
            paused: false,
            scope: Scope::All,
            restore_on_exit: true,
            close_to_tray: true,
            theme: Theme::Auto,
        }
    }
}

impl Settings {
    /// Remet les valeurs dans des bornes sures et retire les doublons.
    pub fn sanitize(&mut self) {
        self.desktop = self.desktop.clamp(0, 100);
        self.game_default = self.game_default.clamp(0, 100);
        let mut seen: Vec<String> = Vec::new();
        self.games.retain(|g| {
            let key = g.exe.to_lowercase();
            if key.is_empty() || seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });
        for g in &mut self.games {
            g.percent = g.percent.map(|p| p.clamp(0, 100));
            g.name = g.name.trim().to_string();
            if g.name.is_empty() {
                g.name = g
                    .exe
                    .trim_end_matches(".exe")
                    .trim_end_matches(".EXE")
                    .to_string();
            }
        }
    }

    /// Ramene tous les niveaux dans la plage que le pilote sait reellement appliquer
    /// (ex. 50-100 % avec l'interface DVC classique).
    pub fn clamp_to(&mut self, min: i32, max: i32) {
        if max <= min {
            return;
        }
        self.desktop = self.desktop.clamp(min, max);
        self.game_default = self.game_default.clamp(min, max);
        for g in &mut self.games {
            g.percent = g.percent.map(|p| p.clamp(min, max));
        }
    }
}

/// Correspondance lineaire entre l'echelle native du pilote et des pourcents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scale {
    pub n_min: i32,
    pub n_max: i32,
    pub p_min: i32,
    pub p_max: i32,
}

impl Scale {
    pub fn to_native(&self, percent: i32) -> i32 {
        let p = percent.clamp(self.p_min, self.p_max);
        if self.p_max == self.p_min {
            return self.n_min;
        }
        let t = (p - self.p_min) as f64 / (self.p_max - self.p_min) as f64;
        (self.n_min as f64 + t * (self.n_max - self.n_min) as f64).round() as i32
    }

    pub fn to_percent(&self, native: i32) -> i32 {
        let n = native.clamp(self.n_min.min(self.n_max), self.n_max.max(self.n_min));
        if self.n_max == self.n_min {
            return self.p_min;
        }
        let t = (n - self.n_min) as f64 / (self.n_max - self.n_min) as f64;
        (self.p_min as f64 + t * (self.p_max - self.p_min) as f64).round() as i32
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub available: bool,
    pub name: String,
    /// Ex. « NVAPI · DVC Ex » ou « NVAPI · DVC classique ».
    pub api: String,
    pub min_percent: i32,
    pub max_percent: i32,
    pub default_percent: i32,
    pub displays: usize,
    pub error: Option<String>,
}

impl GpuInfo {
    pub fn unavailable(error: impl Into<String>) -> Self {
        Self {
            available: false,
            name: String::new(),
            api: "aucune".into(),
            min_percent: 0,
            max_percent: 100,
            default_percent: 50,
            displays: 0,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub active_game_id: Option<String>,
    pub active_game_name: Option<String>,
    pub previewing: bool,
    /// Niveau actuellement applique au premier ecran.
    pub applied_percent: Option<i32>,
    pub error: Option<String>,
}

/// Fenetre au premier plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Fg {
    pub exe: String,
    pub path: Option<String>,
    pub title: String,
    /// Nom de l'ecran Windows (`\\.\DISPLAY1`).
    pub monitor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub exe: String,
    pub title: String,
    pub path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_classic_maps_0_63_to_50_100() {
        let s = Scale {
            n_min: 0,
            n_max: 63,
            p_min: 50,
            p_max: 100,
        };
        assert_eq!(s.to_native(50), 0);
        assert_eq!(s.to_native(100), 63);
        assert_eq!(s.to_native(75), 32); // 31.5 arrondi
        assert_eq!(s.to_native(10), 0); // sous le minimum : borne
        assert_eq!(s.to_percent(0), 50);
        assert_eq!(s.to_percent(63), 100);
    }

    #[test]
    fn scale_ex_is_identity_on_0_100() {
        let s = Scale {
            n_min: 0,
            n_max: 100,
            p_min: 0,
            p_max: 100,
        };
        for p in [0, 1, 50, 99, 100] {
            assert_eq!(s.to_native(p), p);
            assert_eq!(s.to_percent(p), p);
        }
    }

    #[test]
    fn sanitize_clamps_and_dedupes() {
        let g = |exe: &str, p: Option<i32>| Game {
            id: exe.into(),
            name: exe.into(),
            exe: exe.into(),
            path: None,
            percent: p,
            enabled: true,
        };
        let mut s = Settings {
            desktop: 250,
            game_default: -4,
            games: vec![g("cs2.exe", Some(400)), g("CS2.EXE", None), g("", None)],
            ..Settings::default()
        };
        s.sanitize();
        assert_eq!((s.desktop, s.game_default), (100, 0));
        assert_eq!(s.games.len(), 1);
        assert_eq!(s.games[0].percent, Some(100));
    }

    #[test]
    fn clamp_to_driver_range() {
        let mut s = Settings {
            desktop: 20,
            game_default: 100,
            ..Settings::default()
        };
        s.games.push(Game {
            id: "a".into(),
            name: "  ".into(),
            exe: "cs2.exe".into(),
            path: None,
            percent: Some(10),
            enabled: true,
        });
        s.sanitize();
        s.clamp_to(50, 100);
        assert_eq!((s.desktop, s.game_default), (50, 100));
        assert_eq!(s.games[0].percent, Some(50));
        assert_eq!(
            s.games[0].name, "cs2",
            "nom vide remplace par le nom du fichier"
        );
        // Plage invalide : aucun changement.
        s.desktop = 10;
        s.clamp_to(0, 0);
        assert_eq!(s.desktop, 10);
    }

    #[test]
    fn settings_accept_partial_json() {
        let s: Settings = serde_json::from_str(r#"{"desktop": 60}"#).unwrap();
        assert_eq!(s.desktop, 60);
        assert_eq!(s.game_default, 80);
        assert!(s.close_to_tray);
    }
}
