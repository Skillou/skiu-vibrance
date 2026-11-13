//! Vibrance numerique NVIDIA via NVAPI, charge dynamiquement depuis le pilote
//! (`nvapi64.dll`). Aucun fichier a installer en plus du pilote graphique.
//!
//! Strategie : on prefere `NvAPI_Get/SetDVCLevelEx` (echelle 0-100 % comme le Panneau
//! NVIDIA) et on retombe sur `NvAPI_Get/SetDVCLevel` (0-63, soit 50-100 %) si
//! l'interface etendue n'est pas disponible sur ce pilote.

use core::ffi::c_void;
use core::mem::{size_of, transmute_copy};
use std::ffi::CString;

use skiu_core::{Dvc, GpuInfo, Scale};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

type Status = i32;
type Handle = *mut c_void;

// Identifiants d'interface NVAPI (pour nvapi_QueryInterface).
const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_UNLOAD: u32 = 0xD22B_DD7E;
const ID_ENUM_DISPLAY: u32 = 0x9ABD_D40D;
const ID_ENUM_GPU: u32 = 0xE5AC_921F;
const ID_GPU_NAME: u32 = 0xCEEE_8E9F;
const ID_ASSOC_HANDLE: u32 = 0x35C2_9134;
const ID_ASSOC_OUTPUT: u32 = 0xD995_937E;
const ID_GET_DVC: u32 = 0x4085_DE45;
const ID_SET_DVC: u32 = 0x1724_09B4;
const ID_GET_DVC_EX: u32 = 0x0E45_002D;
const ID_SET_DVC_EX: u32 = 0x4A82_C2B1;
const ID_ERROR_MESSAGE: u32 = 0x6C2D_048C;

#[repr(C)]
#[derive(Clone, Copy)]
struct DvcInfo {
    version: u32,
    current: i32,
    min: i32,
    max: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DvcInfoEx {
    version: u32,
    current: i32,
    min: i32,
    max: i32,
    default: i32,
}

const fn make_version(size: usize, ver: u32) -> u32 {
    size as u32 | (ver << 16)
}

type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *mut c_void;
type InitFn = unsafe extern "C" fn() -> Status;
type UnloadFn = unsafe extern "C" fn() -> Status;
type EnumDisplayFn = unsafe extern "C" fn(u32, *mut Handle) -> Status;
type EnumGpuFn = unsafe extern "C" fn(*mut Handle, *mut u32) -> Status;
type GpuNameFn = unsafe extern "C" fn(Handle, *mut u8) -> Status;
type AssocHandleFn = unsafe extern "C" fn(*const u8, *mut Handle) -> Status;
type AssocOutputFn = unsafe extern "C" fn(Handle, *mut u32) -> Status;
type GetDvcFn = unsafe extern "C" fn(Handle, u32, *mut DvcInfo) -> Status;
type SetDvcFn = unsafe extern "C" fn(Handle, u32, u32) -> Status;
type GetDvcExFn = unsafe extern "C" fn(Handle, u32, *mut DvcInfoEx) -> Status;
type SetDvcExFn = unsafe extern "C" fn(Handle, u32, *mut DvcInfoEx) -> Status;
type ErrorMessageFn = unsafe extern "C" fn(Status, *mut u8) -> Status;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Api {
    Ex,
    Classic,
}

struct Fns {
    unload: UnloadFn,
    enum_display: EnumDisplayFn,
    assoc_handle: Option<AssocHandleFn>,
    assoc_output: Option<AssocOutputFn>,
    get_dvc: Option<GetDvcFn>,
    set_dvc: Option<SetDvcFn>,
    get_dvc_ex: Option<GetDvcExFn>,
    set_dvc_ex: Option<SetDvcExFn>,
    error_message: Option<ErrorMessageFn>,
}

pub struct Nvapi {
    fns: Fns,
    api: Api,
    scale: Scale,
    default_percent: i32,
    gpu_name: String,
}

/// Convertit un pointeur d'interface NVAPI en pointeur de fonction type.
///
/// # Safety
/// `T` doit etre un type pointeur de fonction de la taille d'un pointeur.
unsafe fn iface<T: Copy>(qi: QueryInterfaceFn, id: u32) -> Option<T> {
    let p = qi(id);
    if p.is_null() {
        None
    } else {
        debug_assert_eq!(size_of::<T>(), size_of::<*mut c_void>());
        Some(transmute_copy::<*mut c_void, T>(&p))
    }
}

impl Nvapi {
    pub fn load() -> Result<Self, String> {
        unsafe {
            let dll = if cfg!(target_pointer_width = "64") {
                "nvapi64.dll"
            } else {
                "nvapi.dll"
            };
            let wide: Vec<u16> = dll.encode_utf16().chain(Some(0)).collect();
            let lib = LoadLibraryW(wide.as_ptr());
            if lib.is_null() {
                return Err(format!(
                    "{dll} introuvable : aucun pilote NVIDIA installe ?"
                ));
            }
            let qi_ptr = GetProcAddress(lib, c"nvapi_QueryInterface".as_ptr().cast());
            let Some(qi_ptr) = qi_ptr else {
                return Err("nvapi_QueryInterface absent du pilote".into());
            };
            let qi: QueryInterfaceFn = transmute_copy(&qi_ptr);

            let init: InitFn = iface(qi, ID_INITIALIZE).ok_or("NvAPI_Initialize indisponible")?;
            let st = init();
            if st != 0 {
                return Err(format!("NvAPI_Initialize a echoue (code {st})"));
            }
            let fns = Fns {
                unload: iface(qi, ID_UNLOAD).ok_or("NvAPI_Unload indisponible")?,
                enum_display: iface(qi, ID_ENUM_DISPLAY)
                    .ok_or("NvAPI_EnumNvidiaDisplayHandle indisponible")?,
                assoc_handle: iface(qi, ID_ASSOC_HANDLE),
                assoc_output: iface(qi, ID_ASSOC_OUTPUT),
                get_dvc: iface(qi, ID_GET_DVC),
                set_dvc: iface(qi, ID_SET_DVC),
                get_dvc_ex: iface(qi, ID_GET_DVC_EX),
                set_dvc_ex: iface(qi, ID_SET_DVC_EX),
                error_message: iface(qi, ID_ERROR_MESSAGE),
            };

            let gpu_name = gpu_name(qi, &fns).unwrap_or_else(|| "GPU NVIDIA".to_string());
            let mut me = Nvapi {
                fns,
                api: Api::Classic,
                scale: Scale {
                    n_min: 0,
                    n_max: 63,
                    p_min: 50,
                    p_max: 100,
                },
                default_percent: 50,
                gpu_name,
            };

            let displays = me.displays();
            let Some(&first) = displays.first() else {
                return Err(
                    "Aucun ecran NVIDIA detecte (ecran branche sur la carte NVIDIA ?)".into(),
                );
            };
            me.detect_api(first)?;
            Ok(me)
        }
    }

    fn describe(&self, status: Status) -> String {
        if let Some(f) = self.fns.error_message {
            let mut buf = [0u8; 64];
            unsafe {
                if f(status, buf.as_mut_ptr()) == 0 {
                    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
                    let msg = String::from_utf8_lossy(&buf[..end]).trim().to_string();
                    if !msg.is_empty() {
                        return format!("{msg} (code {status})");
                    }
                }
            }
        }
        format!("erreur NVAPI {status}")
    }

    fn output_id(&self, display: usize) -> u32 {
        let mut id = 0u32;
        if let Some(f) = self.fns.assoc_output {
            let st = unsafe { f(display as Handle, &mut id) };
            if st != 0 {
                return 0;
            }
        }
        id
    }

    /// Choisit l'interface DVC disponible et en deduit l'echelle.
    fn detect_api(&mut self, display: usize) -> Result<(), String> {
        let out = self.output_id(display);
        let h = display as Handle;
        let mut last = None;

        if let Some(get_ex) = self.fns.get_dvc_ex {
            let mut info = DvcInfoEx {
                version: make_version(size_of::<DvcInfoEx>(), 1),
                current: 0,
                min: 0,
                max: 0,
                default: 0,
            };
            let st = unsafe { get_ex(h, out, &mut info) };
            if st == 0 && info.max > info.min {
                self.api = Api::Ex;
                self.scale = Scale {
                    n_min: info.min,
                    n_max: info.max,
                    p_min: 0,
                    p_max: 100,
                };
                self.default_percent = self.scale.to_percent(info.default);
                return Ok(());
            }
            last = Some(st);
        }
        if let Some(get) = self.fns.get_dvc {
            let mut info = DvcInfo {
                version: make_version(size_of::<DvcInfo>(), 1),
                current: 0,
                min: 0,
                max: 0,
            };
            let st = unsafe { get(h, out, &mut info) };
            if st == 0 && info.max > info.min {
                self.api = Api::Classic;
                self.scale = Scale {
                    n_min: info.min,
                    n_max: info.max,
                    p_min: 50,
                    p_max: 100,
                };
                self.default_percent = 50;
                return Ok(());
            }
            last = Some(st);
        }
        Err(match last {
            Some(st) => format!("Vibrance numerique indisponible : {}", self.describe(st)),
            None => "Ce pilote n'expose pas la vibrance numerique (DVC)".into(),
        })
    }
}

unsafe fn gpu_name(qi: QueryInterfaceFn, _fns: &Fns) -> Option<String> {
    let enum_gpu: EnumGpuFn = iface(qi, ID_ENUM_GPU)?;
    let name_fn: GpuNameFn = iface(qi, ID_GPU_NAME)?;
    let mut handles = [core::ptr::null_mut::<c_void>(); 64];
    let mut count = 0u32;
    if enum_gpu(handles.as_mut_ptr(), &mut count) != 0 || count == 0 {
        return None;
    }
    let mut buf = [0u8; 64];
    if name_fn(handles[0], buf.as_mut_ptr()) != 0 {
        return None;
    }
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    let s = String::from_utf8_lossy(&buf[..end]).trim().to_string();
    (!s.is_empty()).then_some(s)
}

impl Dvc for Nvapi {
    fn info(&self) -> GpuInfo {
        GpuInfo {
            available: true,
            name: self.gpu_name.clone(),
            api: match self.api {
                Api::Ex => "NVAPI · DVC étendu".into(),
                Api::Classic => "NVAPI · DVC classique".into(),
            },
            min_percent: self.scale.p_min,
            max_percent: self.scale.p_max,
            default_percent: self.default_percent,
            displays: self.displays().len(),
            error: None,
        }
    }

    fn displays(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for i in 0..32u32 {
            let mut h: Handle = core::ptr::null_mut();
            let st = unsafe { (self.fns.enum_display)(i, &mut h) };
            if st != 0 || h.is_null() {
                break;
            }
            out.push(h as usize);
        }
        out
    }

    fn display_for_monitor(&self, device_name: &str) -> Option<usize> {
        let f = self.fns.assoc_handle?;
        let name = CString::new(device_name).ok()?;
        let mut h: Handle = core::ptr::null_mut();
        let st = unsafe { f(name.as_ptr() as *const u8, &mut h) };
        (st == 0 && !h.is_null()).then_some(h as usize)
    }

    fn get_percent(&mut self, display: usize) -> Result<i32, String> {
        let out = self.output_id(display);
        let h = display as Handle;
        unsafe {
            match self.api {
                Api::Ex => {
                    let f = self.fns.get_dvc_ex.ok_or("GetDVCInfoEx indisponible")?;
                    let mut info = DvcInfoEx {
                        version: make_version(size_of::<DvcInfoEx>(), 1),
                        current: 0,
                        min: 0,
                        max: 0,
                        default: 0,
                    };
                    let st = f(h, out, &mut info);
                    if st != 0 {
                        return Err(self.describe(st));
                    }
                    Ok(self.scale.to_percent(info.current))
                }
                Api::Classic => {
                    let f = self.fns.get_dvc.ok_or("GetDVCInfo indisponible")?;
                    let mut info = DvcInfo {
                        version: make_version(size_of::<DvcInfo>(), 1),
                        current: 0,
                        min: 0,
                        max: 0,
                    };
                    let st = f(h, out, &mut info);
                    if st != 0 {
                        return Err(self.describe(st));
                    }
                    Ok(self.scale.to_percent(info.current))
                }
            }
        }
    }

    fn set_percent(&mut self, display: usize, percent: i32) -> Result<(), String> {
        let out = self.output_id(display);
        let h = display as Handle;
        let native = self.scale.to_native(percent);
        unsafe {
            match self.api {
                Api::Ex => {
                    let f = self.fns.set_dvc_ex.ok_or("SetDVCLevelEx indisponible")?;
                    let mut info = DvcInfoEx {
                        version: make_version(size_of::<DvcInfoEx>(), 1),
                        current: native,
                        min: self.scale.n_min,
                        max: self.scale.n_max,
                        default: self.scale.to_native(self.default_percent),
                    };
                    let st = f(h, out, &mut info);
                    if st != 0 {
                        return Err(self.describe(st));
                    }
                }
                Api::Classic => {
                    let f = self.fns.set_dvc.ok_or("SetDVCLevel indisponible")?;
                    let st = f(h, out, native.max(0) as u32);
                    if st != 0 {
                        return Err(self.describe(st));
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for Nvapi {
    fn drop(&mut self) {
        unsafe {
            (self.fns.unload)();
        }
    }
}
