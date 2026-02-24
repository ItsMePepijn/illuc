use anyhow::{bail, Context};
use std::collections::HashMap;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::process::Command;
use tauri::Manager;

const SETTINGS_FILE_NAME: &str = "settings.toml";
const THEMES_DIR_NAME: &str = "themes";
const DEFAULT_SETTINGS: &str = include_str!("default_settings.toml");
const DEFAULT_LIGHT_THEME: &str = include_str!("themes/light.toml");
const DEFAULT_DARK_THEME: &str = include_str!("themes/dark.toml");
const DEFAULT_AMBER_MONOCHROME_THEME: &str = include_str!("themes/amber-monochrome.toml");
const DEFAULT_THEME_NAME: &str = "light";
const DEFAULT_SYNTAX_THEME_NAME: &str = "light";
pub mod commands;
pub mod watcher;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThemeSettingsSnapshot {
    pub(crate) syntax_theme: String,
    pub(crate) values: HashMap<String, String>,
}

pub fn resolve_default_theme_name(window_theme: Option<tauri::Result<tauri::Theme>>) -> String {
    let resolved = match window_theme {
        Some(Ok(tauri::Theme::Dark)) => "dark".to_string(),
        Some(Ok(tauri::Theme::Light)) => "light".to_string(),
        Some(Ok(_)) => DEFAULT_THEME_NAME.to_string(),
        _ => DEFAULT_THEME_NAME.to_string(),
    };

    // On Linux, `window.theme()` can report Light even when GNOME is configured as "prefer-dark".
    // Prefer the explicit desktop setting when available.
    #[cfg(target_os = "linux")]
    {
        let mut resolved = resolved;
        if let Some(color_scheme) = gnome_color_scheme() {
            if matches!(color_scheme.as_str(), "prefer-dark" | "dark") {
                resolved = "dark".to_string();
            } else if matches!(color_scheme.as_str(), "default" | "prefer-light" | "light") {
                resolved = "light".to_string();
            }
        } else if let Some(gtk_theme) = gnome_gtk_theme_name() {
            // Older GNOME setups often encode dark preference in the GTK theme name.
            if gtk_theme.to_ascii_lowercase().contains("dark") {
                resolved = "dark".to_string();
            }
        }
        return resolved;
    }

    resolved
}

#[cfg(target_os = "linux")]
fn gnome_color_scheme() -> Option<String> {
    // Returns values like: `'default'` or `'prefer-dark'`.
    let out = match Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            log::warn!("failed to query GNOME color scheme via gsettings: {error}");
            return None;
        }
    };
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(s.trim_matches('\'').trim_matches('"').trim().to_string())
}

#[cfg(target_os = "linux")]
fn gnome_gtk_theme_name() -> Option<String> {
    // Returns values like: `'Adwaita-dark'`.
    let out = match Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            log::warn!("failed to query GNOME GTK theme via gsettings: {error}");
            return None;
        }
    };
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(s.trim_matches('\'').trim_matches('"').trim().to_string())
}

fn resolve_app_config_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    app.path()
        .app_config_dir()
        .with_context(|| "failed to resolve app config dir")
}

pub fn ensure_user_settings_file(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    let config_dir = resolve_app_config_dir(app)?;
    let themes_dir = config_dir.join(THEMES_DIR_NAME);

    if config_dir.exists() && !config_dir.is_dir() {
        bail!("{} exists but is not a directory", config_dir.display());
    }

    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("failed to create {}", config_dir.display()))?;

    let settings_path = config_dir.join(SETTINGS_FILE_NAME);
    if settings_path.exists() && !settings_path.is_file() {
        bail!("{} exists but is not a file", settings_path.display());
    }
    if !settings_path.exists() {
        std::fs::write(&settings_path, DEFAULT_SETTINGS)
            .with_context(|| format!("failed to write {}", settings_path.display()))?;
    }

    if themes_dir.exists() && !themes_dir.is_dir() {
        bail!("{} exists but is not a directory", themes_dir.display());
    }
    std::fs::create_dir_all(&themes_dir)
        .with_context(|| format!("failed to create {}", themes_dir.display()))?;

    ensure_default_theme_file(&themes_dir, "light", DEFAULT_LIGHT_THEME)?;
    ensure_default_theme_file(&themes_dir, "dark", DEFAULT_DARK_THEME)?;
    ensure_default_theme_file(&themes_dir, "amber-monochrome", DEFAULT_AMBER_MONOCHROME_THEME)?;

    Ok(settings_path)
}

pub fn load_theme_settings(
    app: &tauri::AppHandle,
    default_theme_name: &str,
) -> anyhow::Result<HashMap<String, String>> {
    let theme_table = load_selected_theme_table(app, default_theme_name)?;

    let mut result = HashMap::new();
    flatten_theme_table("", &theme_table, &mut result);

    Ok(result)
}

pub fn load_selected_syntax_theme_name(
    app: &tauri::AppHandle,
    default_theme_name: &str,
) -> anyhow::Result<String> {
    let theme_table = load_selected_theme_table(app, default_theme_name)?;
    Ok(extract_syntax_theme_name(&theme_table))
}

pub(crate) fn load_theme_settings_snapshot(
    app: &tauri::AppHandle,
    default_theme_name: &str,
) -> anyhow::Result<ThemeSettingsSnapshot> {
    let theme_table = load_selected_theme_table(app, default_theme_name)?;
    let syntax_theme = extract_syntax_theme_name(&theme_table);

    let mut values = HashMap::new();
    flatten_theme_table("", &theme_table, &mut values);

    Ok(ThemeSettingsSnapshot {
        syntax_theme,
        values,
    })
}

fn extract_syntax_theme_name(theme_table: &toml::map::Map<String, toml::Value>) -> String {
    let syntax_theme = theme_table
        .get("git")
        .and_then(toml::Value::as_table)
        .and_then(|git| git.get("diff"))
        .and_then(toml::Value::as_table)
        .and_then(|diff| diff.get("syntax_theme"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_SYNTAX_THEME_NAME);

    if is_valid_syntax_theme_name(syntax_theme) {
        syntax_theme.to_string()
    } else {
        DEFAULT_SYNTAX_THEME_NAME.to_string()
    }
}

fn load_selected_theme_table(
    app: &tauri::AppHandle,
    default_theme_name: &str,
) -> anyhow::Result<toml::map::Map<String, toml::Value>> {
    let settings_path = ensure_user_settings_file(app)?;
    let content = std::fs::read_to_string(&settings_path)
        .with_context(|| format!("failed to read {}", settings_path.display()))?;
    let parsed: toml::Value = content
        .parse()
        .with_context(|| format!("failed to parse {}", settings_path.display()))?;

    let theme_name = parsed
        .get("theme")
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_theme_name);

    let themes_dir = resolve_app_config_dir(app)?.join(THEMES_DIR_NAME);
    let selected_theme_name = if is_valid_theme_name(theme_name) {
        theme_name
    } else if is_valid_theme_name(default_theme_name) {
        default_theme_name
    } else {
        DEFAULT_THEME_NAME
    };

    let theme_path = resolve_theme_path(&themes_dir, selected_theme_name)?;
    let selected_theme_path = if theme_path.exists() {
        theme_path
    } else {
        resolve_theme_path(&themes_dir, DEFAULT_THEME_NAME)?
    };
    let theme_content = std::fs::read_to_string(&selected_theme_path)
        .with_context(|| format!("failed to read {}", selected_theme_path.display()))?;
    let theme_value: toml::Value = theme_content
        .parse()
        .with_context(|| format!("failed to parse {}", selected_theme_path.display()))?;

    let Some(theme_table) = theme_value.as_table() else {
        return Ok(toml::map::Map::new());
    };
    Ok(theme_table.clone())
}

fn flatten_theme_table(
    prefix: &str,
    table: &toml::map::Map<String, toml::Value>,
    out: &mut HashMap<String, String>,
) {
    for (key, value) in table {
        // Theme config can contain non-CSS sections; ignore them for the CSS variable payload.
        if prefix.is_empty() && key == "git" {
            continue;
        }

        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };

        match value {
            toml::Value::Table(inner) => flatten_theme_table(&full_key, inner, out),
            toml::Value::String(inner) => {
                out.insert(full_key, inner.clone());
            }
            toml::Value::Integer(inner) => {
                out.insert(full_key, inner.to_string());
            }
            toml::Value::Float(inner) => {
                out.insert(full_key, inner.to_string());
            }
            toml::Value::Boolean(inner) => {
                out.insert(full_key, inner.to_string());
            }
            _ => {}
        }
    }
}

fn ensure_default_theme_file(
    themes_dir: &std::path::Path,
    name: &str,
    body: &str,
) -> anyhow::Result<()> {
    let path = themes_dir.join(format!("{name}.toml"));
    if path.exists() {
        if !path.is_file() {
            bail!("{} exists but is not a file", path.display());
        }
        if cfg!(debug_assertions) {
            std::fs::write(&path, body)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        return Ok(());
    }
    std::fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn resolve_theme_path(themes_dir: &std::path::Path, theme_name: &str) -> anyhow::Result<PathBuf> {
    if !is_valid_theme_name(theme_name) {
        bail!("invalid theme name: {theme_name}");
    }
    Ok(themes_dir.join(format!("{theme_name}.toml")))
}

fn is_valid_theme_name(theme_name: &str) -> bool {
    !theme_name.is_empty()
        && theme_name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn is_valid_syntax_theme_name(syntax_theme: &str) -> bool {
    matches!(syntax_theme, "light" | "dark")
}
