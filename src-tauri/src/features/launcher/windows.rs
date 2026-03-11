#[cfg(target_os = "windows")]
use crate::utils::windows::suppress_console_window;
#[cfg(target_os = "windows")]
use std::env;
use std::path::PathBuf;
use std::process::Command;

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

pub(crate) fn prepare_command(_command: &mut Command) {
    #[cfg(target_os = "windows")]
    suppress_console_window(_command);
}

#[cfg(target_os = "windows")]
pub(crate) fn load_icon_data_url(path: &PathBuf) -> Option<String> {
    let script = r#"
Add-Type -AssemblyName System.Drawing
$icon = [System.Drawing.Icon]::ExtractAssociatedIcon($args[0])
if ($null -eq $icon) { exit 0 }
$bitmap = $icon.ToBitmap()
$stream = New-Object System.IO.MemoryStream
$bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
[Convert]::ToBase64String($stream.ToArray())
"#;

    let mut last_error = None;
    for shell in ["powershell.exe", "pwsh.exe"] {
        let output = Command::new(shell)
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .arg(path)
            .output();

        match output {
            Ok(result) if result.status.success() => {
                let encoded = String::from_utf8(result.stdout).ok()?;
                let encoded = encoded.trim();
                if encoded.is_empty() {
                    return None;
                }
                return Some(format!("data:image/png;base64,{}", encoded));
            }
            Ok(result) => {
                last_error = Some(result.status);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return None,
        }
    }

    let _ = last_error;
    None
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn load_icon_data_url(_: &PathBuf) -> Option<String> {
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
