mod commands;
mod error;
mod features;
mod utils;

use crate::features::launcher::commands::open_file_in_vscode::open_file_in_vscode;
use crate::features::launcher::commands::open_path_in_explorer::open_path_in_explorer;
use crate::features::launcher::commands::open_path_in_vscode::open_path_in_vscode;
use crate::features::launcher::commands::open_path_terminal::open_path_terminal;
use crate::features::settings::commands::settings_theme_get::settings_theme_get;
use crate::features::settings::ensure_user_settings_file;
#[cfg(target_os = "windows")]
use crate::features::shell::native_titlebar::apply_windows_caption_color;
use crate::features::theming::apply_startup_webview_window_css;
use crate::features::theming::apply_startup_window_background;
use crate::features::theming::on_page_load as theming_on_page_load;
use crate::features::tasks::git::commands::task_git_commit::task_git_commit;
use crate::features::tasks::git::commands::task_git_diff_get::task_git_diff_get;
use crate::features::tasks::git::commands::task_git_diff_watch_start::task_git_diff_watch_start;
use crate::features::tasks::git::commands::task_git_diff_watch_stop::task_git_diff_watch_stop;
use crate::features::tasks::git::commands::task_git_has_changes::task_git_has_changes;
use crate::features::tasks::git::commands::task_git_list_branches::task_git_list_branches;
use crate::features::tasks::git::commands::task_git_push::task_git_push;
use crate::features::tasks::management::commands::select_base_repo::select_base_repo;
use crate::features::tasks::management::commands::task_create::task_create;
use crate::features::tasks::management::commands::task_discard::task_discard;
use crate::features::tasks::management::commands::task_load_existing::task_load_existing;
use crate::features::tasks::management::commands::task_open_worktree_in_vscode::task_open_worktree_in_vscode;
use crate::features::tasks::management::commands::task_open_worktree_terminal::task_open_worktree_terminal;
use crate::features::tasks::management::commands::task_start::task_start;
use crate::features::tasks::management::commands::task_stop::task_stop;
use crate::features::tasks::management::commands::task_terminal_resize::task_terminal_resize;
use crate::features::tasks::management::commands::task_terminal_start::task_terminal_start;
use crate::features::tasks::management::commands::task_terminal_write::task_terminal_write;
use crate::features::tasks::review::commands::task_review_add_comment::task_review_add_comment;
use crate::features::tasks::review::commands::task_review_delete_comment::task_review_delete_comment;
use crate::features::tasks::review::commands::task_review_edit_comment::task_review_edit_comment;
use crate::features::tasks::review::commands::task_review_get::task_review_get;
use crate::features::tasks::review::commands::task_review_get_user_display_name::task_review_get_user_display_name;
use crate::features::tasks::review::commands::task_review_update_thread_status::task_review_update_thread_status;
use crate::features::tasks::TaskManager;
use crate::features::time_tracking::commands::task_time_tracking_get::task_time_tracking_get;
use crate::features::time_tracking::commands::task_time_tracking_record::task_time_tracking_record;
use log::info;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    dotenvy::dotenv().ok();
    let _ = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("illuc=debug,tauri=info"),
    )
    .format_timestamp_millis()
    .try_init();
    info!("starting illuc tauri app");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .on_page_load(|webview, payload| {
            theming_on_page_load(webview, payload);
        })
        .setup(|app| {
            match ensure_user_settings_file(&app.handle()) {
                Ok(settings_path) => {
                    let data_dir = settings_path
                        .parent()
                        .map(|path| path.to_path_buf())
                        .unwrap_or_else(|| settings_path.clone());
                    info!("illuc data dir: {}", data_dir.display());
                }
                Err(error) => {
                    log::warn!("failed to initialize user settings file: {error}");
                }
            }

            // Apply an initial native window + webview background color before showing the window
            // to avoid a white flash during startup. This is driven by the selected theme.
            if let Some(window) = app.get_webview_window("main") {
                apply_startup_window_background(&window);
                apply_startup_webview_window_css(&window);

                #[cfg(target_os = "windows")]
                {
                    if let Err(error) = apply_windows_caption_color(&window) {
                        log::warn!("failed to apply native caption color: {error}");
                    }
                }

                if let Err(error) = window.show() {
                    log::warn!("failed to show main window: {error}");
                }
            }
            Ok(())
        })
        .manage(TaskManager::default())
        .invoke_handler(tauri::generate_handler![
            select_base_repo,
            task_create,
            task_start,
            task_stop,
            task_discard,
            task_terminal_write,
            task_terminal_resize,
            task_terminal_start,
            task_git_diff_get,
            task_git_has_changes,
            task_git_diff_watch_start,
            task_git_diff_watch_stop,
            task_git_commit,
            task_git_push,
            task_load_existing,
            task_open_worktree_in_vscode,
            task_open_worktree_terminal,
            open_path_in_vscode,
            open_file_in_vscode,
            open_path_terminal,
            open_path_in_explorer,
            task_git_list_branches,
            task_time_tracking_get,
            task_time_tracking_record,
            task_review_get,
            task_review_add_comment,
            task_review_edit_comment,
            task_review_delete_comment,
            task_review_get_user_display_name,
            task_review_update_thread_status,
            settings_theme_get
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
