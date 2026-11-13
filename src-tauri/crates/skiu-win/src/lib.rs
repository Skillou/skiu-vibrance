//! Backends Windows de Skiu Vibrance :
//! - `nvapi`  : vibrance numerique via `nvapi64.dll` (appels directs, sans DLL tierce) ;
//! - `winsys` : fenetre au premier plan et liste des fenetres (Win32).
//!
//! Sur les autres systemes, `platform()` renvoie des implementations « vides »
//! pour que l'application reste compilable et testable partout.
//! Pour verifier les types Windows depuis Linux :
//! `RUSTFLAGS="--cfg skiu_typecheck" cargo check -p skiu-win`

use skiu_core::{Dvc, Fg, GpuInfo, Sys, WindowInfo};

#[cfg(any(windows, skiu_typecheck))]
mod icon;
#[cfg(any(windows, skiu_typecheck))]
mod nvapi;
#[cfg(any(windows, skiu_typecheck))]
mod winsys;

/// Pilote vide : utilise quand aucune carte NVIDIA n'est exploitable.
pub struct NullDvc(String);

impl NullDvc {
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl Dvc for NullDvc {
    fn info(&self) -> GpuInfo {
        GpuInfo::unavailable(self.0.clone())
    }
    fn displays(&self) -> Vec<usize> {
        Vec::new()
    }
    fn display_for_monitor(&self, _device_name: &str) -> Option<usize> {
        None
    }
    fn get_percent(&mut self, _display: usize) -> Result<i32, String> {
        Err(self.0.clone())
    }
    fn set_percent(&mut self, _display: usize, _percent: i32) -> Result<(), String> {
        Err(self.0.clone())
    }
}

pub struct NullSys;

impl Sys for NullSys {
    fn foreground(&mut self) -> Option<Fg> {
        None
    }
    fn windows(&mut self) -> Vec<WindowInfo> {
        Vec::new()
    }
}

#[cfg(any(windows, skiu_typecheck))]
pub fn platform() -> (Box<dyn Dvc>, Box<dyn Sys>) {
    let dvc: Box<dyn Dvc> = match nvapi::Nvapi::load() {
        Ok(n) => Box::new(n),
        Err(e) => Box::new(NullDvc::new(e)),
    };
    (dvc, Box::new(winsys::WinSys))
}

#[cfg(not(any(windows, skiu_typecheck)))]
pub fn platform() -> (Box<dyn Dvc>, Box<dyn Sys>) {
    (
        Box::new(NullDvc::new("Windows et une carte NVIDIA sont requis")),
        Box::new(NullSys),
    )
}

/// Icone d'un executable sous forme de `data:image/png;base64,...`, prete pour une balise `<img>`.
pub fn exe_icon_data_url(path: &str) -> Option<String> {
    #[cfg(any(windows, skiu_typecheck))]
    {
        let png = icon::exe_icon_png(path, 64)?;
        Some(format!("data:image/png;base64,{}", base64(&png)))
    }
    #[cfg(not(any(windows, skiu_typecheck)))]
    {
        let _ = path;
        None
    }
}

/// Encode des pixels RGBA 8 bits en PNG.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    if rgba.len() != (width * height * 4) as usize {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().ok()?;
        w.write_image_data(rgba).ok()?;
    }
    Some(out)
}

/// Base64 standard (RFC 4648, avec remplissage).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (i, o) in cases {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }

    #[test]
    fn encode_png_produces_valid_header() {
        let png = encode_png(2, 2, &[255; 16]).unwrap();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert!(
            encode_png(2, 2, &[0; 3]).is_none(),
            "taille incoherente refusee"
        );
    }

    #[cfg(windows)]
    #[test]
    fn extracts_icon_from_system_exe() {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let url = exe_icon_data_url(&format!(r"{windir}\explorer.exe"))
            .expect("explorer.exe a une icone");
        assert!(url.starts_with("data:image/png;base64,iVBORw0KGgo"));
        assert!(exe_icon_data_url(r"C:\nexiste\pas.exe").is_none());
    }
}
