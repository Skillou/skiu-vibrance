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
