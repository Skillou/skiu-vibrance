//! Extraction de l'icone d'un executable (ressource Win32) en PNG.

use core::ffi::c_void;
use core::mem::{size_of, zeroed};
use core::ptr::null_mut;

use windows_sys::Win32::{
    Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
    },
    UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO},
};

/// Lit un bitmap GDI en pixels BGRA 32 bits, lignes de haut en bas.
unsafe fn bitmap_bgra(hbm: HBITMAP) -> Option<(u32, u32, Vec<u8>)> {
    let mut bm: BITMAP = zeroed();
    if GetObjectW(
        hbm,
        size_of::<BITMAP>() as i32,
        &mut bm as *mut BITMAP as *mut c_void,
    ) == 0
    {
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight.abs());
    if w <= 0 || h <= 0 || w > 512 || h > 512 {
        return None;
    }
    let mut info: BITMAPINFO = zeroed();
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: w,
        biHeight: -h, // negatif : de haut en bas
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..zeroed()
    };
    let mut buf = vec![0u8; (w * h * 4) as usize];
    let dc = CreateCompatibleDC(null_mut());
    if dc.is_null() {
        return None;
    }
    let lines = GetDIBits(
        dc,
        hbm,
        0,
        h as u32,
        buf.as_mut_ptr() as *mut c_void,
        &mut info,
        DIB_RGB_COLORS,
    );
    DeleteDC(dc);
    (lines == h).then_some((w as u32, h as u32, buf))
}

/// Convertit une icone en RGBA. Les vieilles icones sans canal alpha utilisent le masque.
unsafe fn icon_rgba(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
    let mut ii: ICONINFO = zeroed();
    if GetIconInfo(icon, &mut ii) == 0 {
        return None;
    }
    let color = (!ii.hbmColor.is_null())
        .then(|| bitmap_bgra(ii.hbmColor))
        .flatten();
    let mask = (!ii.hbmMask.is_null())
        .then(|| bitmap_bgra(ii.hbmMask))
        .flatten();
    if !ii.hbmColor.is_null() {
        DeleteObject(ii.hbmColor);
    }
    if !ii.hbmMask.is_null() {
        DeleteObject(ii.hbmMask);
    }
    let (w, h, mut px) = color?;
    let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
    for (i, p) in px.chunks_exact_mut(4).enumerate() {
        p.swap(0, 2); // BGRA -> RGBA
        if !has_alpha {
            // Masque : blanc = transparent.
            let transparent = mask
                .as_ref()
                .is_some_and(|(_, _, m)| m.get(i * 4).copied().unwrap_or(0) != 0);
            p[3] = if transparent { 0 } else { 255 };
        }
    }
    Some((w, h, px))
}

/// Icone principale de `path` en PNG (environ `size` pixels), ou `None` si le fichier n'en a pas.
pub fn exe_icon_png(path: &str, size: u32) -> Option<Vec<u8>> {
    let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    let mut icon: HICON = null_mut();
    let mut id = 0u32;
    let n = unsafe {
        PrivateExtractIconsW(
            wide.as_ptr(),
            0,
            size as i32,
            size as i32,
            &mut icon,
            &mut id,
            1,
            0,
        )
    };
    if n == 0 || n == u32::MAX || icon.is_null() {
        return None;
    }
    let rgba = unsafe { icon_rgba(icon) };
    unsafe { DestroyIcon(icon) };
    let (w, h, px) = rgba?;
    crate::encode_png(w, h, &px)
}
