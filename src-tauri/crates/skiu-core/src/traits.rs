use crate::model::{DisplayId, Fg, GpuInfo, WindowInfo};

/// Acces au pilote graphique (vibrance numerique).
pub trait Dvc: Send {
    fn info(&self) -> GpuInfo;
    /// Ecrans actuellement pilotables (re-enumeres a chaque appel : branchement a chaud).
    fn displays(&self) -> Vec<DisplayId>;
    fn display_for_monitor(&self, device_name: &str) -> Option<DisplayId>;
    fn get_percent(&mut self, display: DisplayId) -> Result<i32, String>;
    fn set_percent(&mut self, display: DisplayId, percent: i32) -> Result<(), String>;
}

/// Acces au systeme : fenetre au premier plan, liste des fenetres.
pub trait Sys: Send {
    fn foreground(&mut self) -> Option<Fg>;
    fn windows(&mut self) -> Vec<WindowInfo>;
}
