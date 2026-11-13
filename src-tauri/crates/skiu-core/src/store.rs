use crate::model::Settings;
use std::{fs, io, path::Path};

/// Lit les reglages ; `None` si le fichier n'existe pas ou est illisible.
///
/// Un fichier present mais invalide (JSON tronque, edition manuelle ratee...) est
/// conserve sous `settings.json.bak` avant d'etre remplace : la liste de jeux de
/// l'utilisateur n'est jamais perdue silencieusement.
pub fn load_settings(path: &Path) -> Option<Settings> {
    let text = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Settings>(text.trim_start_matches('\u{feff}')) {
        Ok(mut s) => {
            s.sanitize();
            Some(s)
        }
        Err(_) => {
            let _ = fs::rename(path, path.with_extension("json.bak"));
            None
        }
    }
}

/// Ecriture atomique (fichier temporaire puis renommage).
pub fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Game, Theme};

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("skiu-test-{tag}-{}", std::process::id()))
    }

    #[test]
    fn roundtrip() {
        let dir = temp_dir("rt");
        let file = dir.join("settings.json");
        let mut s = Settings {
            desktop: 61,
            theme: Theme::Dark,
            ..Settings::default()
        };
        s.games.push(Game {
            id: "a".into(),
            name: "CS2".into(),
            exe: "cs2.exe".into(),
            path: Some("C:\\x\\cs2.exe".into()),
            percent: Some(90),
            enabled: true,
        });
        save_settings(&file, &s).unwrap();
        assert_eq!(load_settings(&file), Some(s));
        assert_eq!(load_settings(&dir.join("absent.json")), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_file_is_backed_up() {
        let dir = temp_dir("bad");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        fs::write(&file, "{\"desktop\": 6").unwrap();
        assert_eq!(load_settings(&file), None);
        assert!(!file.exists());
        assert_eq!(
            fs::read_to_string(dir.join("settings.json.bak")).unwrap(),
            "{\"desktop\": 6"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let dir = temp_dir("bom");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        fs::write(&file, "\u{feff}{\"desktop\": 66}").unwrap();
        assert_eq!(load_settings(&file).map(|s| s.desktop), Some(66));
        let _ = fs::remove_dir_all(dir);
    }
}
