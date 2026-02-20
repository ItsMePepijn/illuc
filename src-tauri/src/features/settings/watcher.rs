use super::{
    ensure_user_settings_file, load_theme_settings_snapshot, resolve_default_theme_name,
    THEMES_DIR_NAME,
};
use crate::utils::file_watcher::is_content_change_event;
use anyhow::{bail, Context};
use log::{debug, warn};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const SETTINGS_THEME_CHANGED_EVENT: &str = "settings_theme_changed";
const WATCH_DEBOUNCE_MS: u64 = 200;

pub fn start_settings_theme_watcher(app: AppHandle) -> anyhow::Result<()> {
    let settings_path = ensure_user_settings_file(&app)?;
    let config_dir = settings_path
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("settings path has no parent"))?;
    let themes_dir = config_dir.join(THEMES_DIR_NAME);
    if themes_dir.exists() && !themes_dir.is_dir() {
        bail!("{} exists but is not a directory", themes_dir.display());
    }

    let default_theme_name =
        resolve_default_theme_name(app.get_webview_window("main").map(|window| window.theme()));
    let initial_snapshot = load_theme_settings_snapshot(&app, &default_theme_name)
        .with_context(|| "failed to load initial valid settings/theme snapshot")?;
    let last_valid = Arc::new(Mutex::new(initial_snapshot));

    let (signal_tx, signal_rx) = mpsc::channel::<()>();
    let watch_settings_path = settings_path.clone();
    let watch_themes_dir = themes_dir.clone();
    let mut watcher =
        notify::recommended_watcher(move |res| match res {
            Ok(event) => {
                if !is_content_change_event(&event) {
                    return;
                }
                if event.paths.iter().any(|path| {
                    should_reload_for_path(path, &watch_settings_path, &watch_themes_dir)
                }) {
                    let _ = signal_tx.send(());
                }
            }
            Err(error) => {
                warn!("settings/theme watch error: {error}");
            }
        })
        .with_context(|| "failed to create settings/theme watcher")?;

    watcher
        .watch(&settings_path, RecursiveMode::NonRecursive)
        .with_context(|| format!("failed to watch {}", settings_path.display()))?;
    watcher
        .watch(&themes_dir, RecursiveMode::Recursive)
        .with_context(|| format!("failed to watch {}", themes_dir.display()))?;

    std::thread::Builder::new()
        .name("settings-theme-watcher".to_string())
        .spawn(move || run_watch_loop(app, watcher, signal_rx, last_valid))
        .with_context(|| "failed to spawn settings/theme watcher thread")?;

    Ok(())
}

fn run_watch_loop(
    app: AppHandle,
    _watcher: RecommendedWatcher,
    signal_rx: mpsc::Receiver<()>,
    last_valid: Arc<Mutex<super::ThemeSettingsSnapshot>>,
) {
    while signal_rx.recv().is_ok() {
        while signal_rx
            .recv_timeout(Duration::from_millis(WATCH_DEBOUNCE_MS))
            .is_ok()
        {}

        let default_theme_name =
            resolve_default_theme_name(app.get_webview_window("main").map(|window| window.theme()));
        let next_snapshot = match load_theme_settings_snapshot(&app, &default_theme_name) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                warn!(
                    "ignoring settings/theme update; keeping previous valid configuration: {error}"
                );
                continue;
            }
        };

        let mut previous = match last_valid.lock() {
            Ok(guard) => guard,
            Err(error) => {
                warn!("failed to lock settings/theme snapshot state: {error}");
                continue;
            }
        };

        if *previous == next_snapshot {
            continue;
        }

        *previous = next_snapshot;
        drop(previous);

        if let Err(error) = app.emit(SETTINGS_THEME_CHANGED_EVENT, ()) {
            warn!("failed to emit {SETTINGS_THEME_CHANGED_EVENT} event: {error}");
        } else {
            debug!("emitted {SETTINGS_THEME_CHANGED_EVENT}");
        }
    }
}

fn should_reload_for_path(
    path: &std::path::Path,
    settings_path: &std::path::Path,
    themes_dir: &std::path::Path,
) -> bool {
    path == settings_path || path.starts_with(themes_dir)
}
