use super::editor::{EditorDefinition, LaunchCommand};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) struct PlatformEditorMetadata {
    pub(crate) launch: LaunchCommand,
    pub(crate) icon_data_url: Option<String>,
}

#[derive(Clone, Debug)]
struct LinuxDesktopEntry {
    file_stem: String,
    name: String,
    exec: String,
    icon: Option<String>,
}

#[cfg(target_os = "linux")]
pub(crate) fn resolve_editor_launch(
    definition: &EditorDefinition,
) -> Option<PlatformEditorMetadata> {
    let desktop_entries = load_linux_desktop_entries();

    for entry in desktop_entries {
        if !matches_linux_desktop_entry(definition, &entry) {
            continue;
        }

        if let Some(launch) = parse_desktop_exec(&entry.exec) {
            return Some(PlatformEditorMetadata {
                icon_data_url: entry
                    .icon
                    .as_deref()
                    .and_then(resolve_icon_path)
                    .and_then(load_icon_data_url),
                launch,
            });
        }
    }

    None
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn resolve_editor_launch(_: &EditorDefinition) -> Option<PlatformEditorMetadata> {
    None
}

#[cfg(target_os = "linux")]
fn matches_linux_desktop_entry(definition: &EditorDefinition, entry: &LinuxDesktopEntry) -> bool {
    let name = entry.name.to_ascii_lowercase();
    let file_stem = entry.file_stem.to_ascii_lowercase();
    let exec = entry.exec.to_ascii_lowercase();
    let exec_name = shell_split(&entry.exec)
        .and_then(|parts| parts.first().cloned())
        .map(|value| {
            Path::new(&value)
                .file_name()
                .map(|segment| segment.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_else(|| value.to_ascii_lowercase())
        })
        .unwrap_or_default();

    definition
        .linux_names
        .iter()
        .any(|candidate| name.contains(candidate))
        || definition
            .linux_desktop_ids
            .iter()
            .any(|candidate| file_stem.contains(candidate))
        || definition
            .linux_execs
            .iter()
            .any(|candidate| exec_name == *candidate || exec.contains(candidate))
}

#[cfg(target_os = "linux")]
fn load_linux_desktop_entries() -> Vec<LinuxDesktopEntry> {
    let mut entries = Vec::new();
    for directory in linux_desktop_directories() {
        let Ok(read_dir) = fs::read_dir(&directory) else {
            continue;
        };

        for item in read_dir.flatten() {
            let path = item.path();
            if path.extension().and_then(|value| value.to_str()) != Some("desktop") {
                continue;
            }

            let Some(entry) = parse_linux_desktop_entry(&path) else {
                continue;
            };
            entries.push(entry);
        }
    }
    entries
}

#[cfg(target_os = "linux")]
fn linux_desktop_directories() -> Vec<PathBuf> {
    let mut directories = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        directories.push(home.join(".local/share/applications"));
        directories.push(home.join(".local/share/flatpak/exports/share/applications"));
        directories.push(home.join(".local/share/JetBrains/Toolbox/apps"));
    }

    directories
}

#[cfg(target_os = "linux")]
fn parse_linux_desktop_entry(path: &Path) -> Option<LinuxDesktopEntry> {
    let contents = fs::read_to_string(path).ok()?;
    let mut in_desktop_entry = false;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut hidden = false;
    let mut no_display = false;
    let mut terminal = false;
    let mut is_application = true;

    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key {
            "Name" => name = Some(value.trim().to_string()),
            "Exec" => exec = Some(value.trim().to_string()),
            "Icon" => icon = Some(value.trim().to_string()),
            "Hidden" => hidden = parse_desktop_bool(value),
            "NoDisplay" => no_display = parse_desktop_bool(value),
            "Terminal" => terminal = parse_desktop_bool(value),
            "Type" => is_application = value.trim().eq_ignore_ascii_case("Application"),
            _ => {}
        }
    }

    if hidden || no_display || terminal || !is_application {
        return None;
    }

    Some(LinuxDesktopEntry {
        file_stem: path.file_stem()?.to_string_lossy().to_string(),
        name: name?,
        exec: exec?,
        icon,
    })
}

#[cfg(target_os = "linux")]
fn parse_desktop_bool(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("true")
}

#[cfg(target_os = "linux")]
fn parse_desktop_exec(exec: &str) -> Option<LaunchCommand> {
    let mut parts = shell_split(exec)?;
    if parts.is_empty() {
        return None;
    }

    parts.retain(|part| !part.contains('%'));
    let command = PathBuf::from(parts.first()?.clone());
    let args = parts.into_iter().skip(1).collect();

    Some(LaunchCommand { command, args })
}

#[cfg(target_os = "linux")]
fn resolve_icon_path(icon: &str) -> Option<PathBuf> {
    let icon_path = Path::new(icon);
    if icon_path.is_absolute() {
        return resolve_icon_file(icon_path);
    }

    let mut candidates = Vec::new();
    for root in icon_search_roots() {
        candidates.push(root.clone().join(icon));
        candidates.push(root.join("pixmaps").join(icon));
    }

    let mut stack = icon_search_roots();
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path
                .file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|stem| stem == icon)
            {
                candidates.push(path);
            }
        }
    }

    candidates
        .into_iter()
        .find_map(|candidate| resolve_icon_file(&candidate))
}

#[cfg(target_os = "linux")]
fn icon_search_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
        PathBuf::from("/var/lib/flatpak/exports/share/icons"),
        PathBuf::from("/var/lib/flatpak/exports/share/app-info/icons"),
    ];

    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share/icons"));
        roots.push(home.join(".icons"));
        roots.push(home.join(".local/share/flatpak/exports/share/icons"));
    }

    roots
}

#[cfg(target_os = "linux")]
fn resolve_icon_file(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        return Some(path.to_path_buf());
    }

    for extension in ["png", "svg", "xpm", "ico"] {
        let candidate = path.with_extension(extension);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn load_icon_data_url(path: PathBuf) -> Option<String> {
    let bytes = fs::read(&path).ok()?;
    let mime_type = match path.extension().and_then(|value| value.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("xpm") => "image/x-xpixmap",
        Some("ico") => "image/x-icon",
        _ => "image/png",
    };

    Some(format!(
        "data:{mime_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

#[cfg(target_os = "linux")]
fn shell_split(value: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            Some(active_quote) if ch == active_quote => {
                quote = None;
            }
            Some(_) if ch == '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            None if ch == '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            None => current.push(ch),
        }
    }

    if quote.is_some() {
        return None;
    }

    if !current.is_empty() {
        parts.push(current);
    }

    Some(parts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn shell_split_handles_quotes_and_escapes() {
        let parts = shell_split(r#"env FOO=bar "/opt/Code - Insiders/bin/code" --reuse-window %F"#)
            .expect("desktop exec should parse");
        assert_eq!(
            parts,
            vec![
                "env",
                "FOO=bar",
                "/opt/Code - Insiders/bin/code",
                "--reuse-window",
                "%F"
            ]
        );
    }

    #[test]
    fn parse_desktop_exec_removes_field_codes() {
        let launch =
            parse_desktop_exec(r#"code --unity-launch %F"#).expect("desktop exec should parse");
        assert_eq!(launch.command.as_os_str(), OsString::from("code"));
        assert_eq!(launch.args, vec!["--unity-launch"]);
    }
}
