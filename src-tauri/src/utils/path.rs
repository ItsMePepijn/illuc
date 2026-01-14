use std::path::Path;

pub fn normalize_path_string(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    normalize_windows_prefix(value)
}

fn normalize_windows_prefix(value: String) -> String {
    #[cfg(target_os = "windows")]
    {
        return strip_windows_prefix(value);
    }
    #[cfg(not(target_os = "windows"))]
    {
        value
    }
}

#[cfg(target_os = "windows")]
fn strip_windows_prefix(mut value: String) -> String {
    if let Some(stripped) = value.strip_prefix(r"\\?\") {
        value = stripped.to_string();
        if let Some(rest) = value.strip_prefix("UNC\\") {
            return format!(r"\\{}", rest);
        }
        return value;
    }
    if let Some(stripped) = value.strip_prefix("//?/") {
        value = stripped.to_string();
        if let Some(rest) = value.strip_prefix("UNC/") {
            return format!("//{}", rest);
        }
        return value;
    }
    value
}
