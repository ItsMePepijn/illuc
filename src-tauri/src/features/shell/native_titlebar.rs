#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
#[cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
};

#[cfg(target_os = "windows")]
fn rgb_to_colorref(red: u8, green: u8, blue: u8) -> u32 {
    u32::from(red) | (u32::from(green) << 8) | (u32::from(blue) << 16)
}

#[cfg(target_os = "windows")]
fn set_dwm_color_attribute(hwnd: isize, attribute: u32, color: u32) -> Result<(), String> {
    let status = unsafe {
        DwmSetWindowAttribute(
            hwnd as _,
            attribute,
            &color as *const u32 as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    };
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "DwmSetWindowAttribute failed with HRESULT {status:#x}"
        ))
    }
}

#[cfg(target_os = "windows")]
pub fn apply_windows_caption_color<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("Unable to get native window handle: {error}"))?;
    let hwnd = match handle.as_raw() {
        RawWindowHandle::Win32(win32) => win32.hwnd.get(),
        _ => return Err("Window handle is not Win32".to_string()),
    };

    // Matches frontend theme surface/text tones.
    let caption_color = rgb_to_colorref(0xf7, 0xf3, 0xec);
    let text_color = rgb_to_colorref(0x60, 0x5a, 0x52);
    set_dwm_color_attribute(hwnd, DWMWA_CAPTION_COLOR as u32, caption_color)?;
    set_dwm_color_attribute(hwnd, DWMWA_TEXT_COLOR as u32, text_color)?;
    Ok(())
}
