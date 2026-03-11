#[cfg(target_os = "windows")]
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
#[cfg(target_os = "windows")]
use base64::Engine as _;
#[cfg(target_os = "windows")]
use png::{BitDepth, ColorType, Encoder};
#[cfg(target_os = "windows")]
use crate::utils::windows::suppress_console_window;
#[cfg(target_os = "windows")]
use std::env;
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::fs;
#[cfg(target_os = "windows")]
use std::io::Cursor;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, RGBQUAD, BI_RGB,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::ExtractIconExW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, HICON, DI_NORMAL};

#[cfg(target_os = "windows")]
pub(crate) fn resolve_install_path(paths: &[&str]) -> Option<PathBuf> {
    paths
        .iter()
        .filter_map(|path| expand_windows_env_vars(path))
        .find(|path| path.is_file())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn resolve_install_path(_: &[&str]) -> Option<PathBuf> {
    None
}

#[cfg(target_os = "windows")]
pub(crate) fn icon_source_from_command(path: &Path) -> Option<PathBuf> {
    let is_exe = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));

    if is_exe {
        return Some(path.to_path_buf());
    }

    extract_windows_exe_path_from_script(path)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn icon_source_from_command(_: &Path) -> Option<PathBuf> {
    None
}

pub(crate) fn prepare_command(_command: &mut Command) {
    #[cfg(target_os = "windows")]
    suppress_console_window(_command);
}

#[cfg(target_os = "windows")]
pub(crate) fn load_icon_data_url(path: &Path) -> Option<String> {
    let rgba = extract_icon_rgba(path, 32)?;
    let mut png_bytes = Cursor::new(Vec::new());
    let mut encoder = Encoder::new(&mut png_bytes, 32, 32);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    let mut writer = encoder.write_header().ok()?;
    writer.write_image_data(&rgba).ok()?;
    drop(writer);

    Some(format!(
        "data:image/png;base64,{}",
        BASE64_STANDARD.encode(png_bytes.into_inner())
    ))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn load_icon_data_url(_: &Path) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
fn expand_windows_env_vars(value: &str) -> Option<PathBuf> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            result.push(ch);
            continue;
        }

        let mut var_name = String::new();
        while let Some(next) = chars.peek().copied() {
            chars.next();
            if next == '%' {
                break;
            }
            var_name.push(next);
        }

        if var_name.is_empty() {
            result.push('%');
            continue;
        }

        let value = env::var(&var_name).ok()?;
        result.push_str(&value);
    }

    Some(PathBuf::from(result))
}

#[cfg(target_os = "windows")]
fn extract_windows_exe_path_from_script(path: &Path) -> Option<PathBuf> {
    let contents = fs::read_to_string(path).ok()?;

    contents
        .split('"')
        .find(|segment| is_windows_exe_path(segment))
        .map(PathBuf::from)
        .filter(|candidate| candidate.is_file())
}

#[cfg(target_os = "windows")]
fn is_windows_exe_path(value: &str) -> bool {
    value.len() > 6
        && value
            .as_bytes()
            .get(1)
            .is_some_and(|separator| *separator == b':')
        && value.ends_with(".exe")
}

#[cfg(target_os = "windows")]
fn extract_icon_rgba(path: &Path, size: i32) -> Option<Vec<u8>> {
    let wide_path = to_wide(path.as_os_str());
    let mut icon_handle: HICON = std::ptr::null_mut();

    let extracted = unsafe {
        ExtractIconExW(
            wide_path.as_ptr(),
            0,
            std::ptr::null_mut(),
            &mut icon_handle,
            1,
        )
    };

    if extracted == 0 || icon_handle.is_null() {
        return None;
    }

    let screen_dc = unsafe { GetDC(std::ptr::null_mut()) };
    if screen_dc.is_null() {
        unsafe {
            DestroyIcon(icon_handle);
        }
        return None;
    }

    let memory_dc = unsafe { CreateCompatibleDC(screen_dc) };
    if memory_dc.is_null() {
        unsafe {
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            DestroyIcon(icon_handle);
        }
        return None;
    }

    let bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size,
            biHeight: -size,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD {
            rgbBlue: 0,
            rgbGreen: 0,
            rgbRed: 0,
            rgbReserved: 0,
        }],
    };

    let mut pixels_ptr = std::ptr::null_mut();
    let dib = unsafe {
        CreateDIBSection(
            memory_dc,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut pixels_ptr,
            std::ptr::null_mut(),
            0,
        )
    };

    if dib.is_null() || pixels_ptr.is_null() {
        unsafe {
            DeleteDC(memory_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            DestroyIcon(icon_handle);
        }
        return None;
    }

    let previous_bitmap = unsafe { SelectObject(memory_dc, dib) };
    let draw_result = unsafe {
        DrawIconEx(
            memory_dc,
            0,
            0,
            icon_handle,
            size,
            size,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        )
    };

    let pixels_len = (size as usize) * (size as usize) * 4;
    let rgba = if draw_result != 0 {
        let bgra = unsafe { std::slice::from_raw_parts(pixels_ptr.cast::<u8>(), pixels_len) };
        Some(convert_bgra_to_rgba(bgra))
    } else {
        None
    };

    unsafe {
        SelectObject(memory_dc, previous_bitmap);
        DeleteObject(dib);
        DeleteDC(memory_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);
        DestroyIcon(icon_handle);
    }

    rgba
}

#[cfg(target_os = "windows")]
fn convert_bgra_to_rgba(bgra: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for pixel in bgra.chunks_exact(4) {
        rgba.push(pixel[2]);
        rgba.push(pixel[1]);
        rgba.push(pixel[0]);
        rgba.push(pixel[3]);
    }
    rgba
}

#[cfg(target_os = "windows")]
fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}
