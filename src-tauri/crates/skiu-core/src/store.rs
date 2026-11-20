use crate::model::Settings;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

/// Lit les reglages ; `None` si le fichier n'existe pas ou est illisible.
///
/// Un fichier present mais invalide (JSON tronque, edition manuelle ratee...) est
/// conserve sous `settings.json.bak` avant d'etre remplace : la liste de jeux de
/// l'utilisateur n'est jamais perdue silencieusement.
pub fn load_settings(path: &Path) -> Option<Settings> {
    // Lecture en octets : un fichier non UTF-8 est lui aussi « invalide », pas « absent ».
    let bytes = fs::read(path).ok()?;
    let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    match serde_json::from_slice::<Settings>(body) {
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

/// Ecriture atomique et durable : fichier temporaire vide sur le disque, puis renommage.
/// Sans `sync_all`, une coupure de courant juste apres le renommage peut laisser un
/// fichier vide, que `load_settings` traiterait comme corrompu.
pub fn save_settings(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    let written = fs::File::create(&tmp).and_then(|mut f| {
        f.write_all(json.as_bytes())?;
        f.sync_all()
    });
    if let Err(e) = written.and_then(|()| fs::rename(&tmp, path)) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
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
    fn non_utf8_file_is_backed_up() {
        let dir = temp_dir("bin");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        fs::write(&file, [0xff, 0xfe, 0x00, 0x7b]).unwrap();
        assert_eq!(load_settings(&file), None);
        assert!(dir.join("settings.json.bak").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn failed_save_leaves_previous_file_and_no_temp() {
        let dir = temp_dir("fail");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let s = Settings::default();
        save_settings(&file, &s).unwrap();
        // Un dossier a la place du fichier temporaire : l'ecriture echoue.
        fs::create_dir_all(dir.join("settings.json.tmp")).unwrap();
        let changed = Settings {
            desktop: 77,
            ..Settings::default()
        };
        assert!(save_settings(&file, &changed).is_err());
        assert_eq!(load_settings(&file), Some(s), "ancien fichier intact");
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
