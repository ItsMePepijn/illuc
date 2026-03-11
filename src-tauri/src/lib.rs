#![allow(dependency_on_unit_never_type_fallback)]

mod commands;
mod error;
mod features;
mod utils;

use crate::features::launcher::commands::open_file_in_vscode::open_file_in_vscode;
use crate::features::launcher::commands::open_path_in_explorer::open_path_in_explorer;
use crate::features::launcher::commands::open_path_in_vscode::open_path_in_vscode;
use crate::features::launcher::commands::open_path_terminal::open_path_terminal;
use crate::features::settings::commands::settings_open_in_vscode::settings_open_in_vscode;
use crate::features::settings::commands::settings_theme_get::settings_theme_get;
use crate::features::settings::ensure_user_settings_file;
use crate::features::settings::watcher::start_settings_theme_watcher;
#[cfg(target_os = "windows")]
use crate::features::shell::native_titlebar::apply_windows_caption_color;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_compact::task_agent_gui_compact;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_interrupt::task_agent_gui_interrupt;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_models::task_agent_gui_models;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_new_chat::task_agent_gui_new_chat;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_request_respond::task_agent_gui_request_respond;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_rollback::task_agent_gui_rollback;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_send::task_agent_gui_send;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_usage::task_agent_gui_usage;
use crate::features::tasks::agents::agent_tui::commands::task_agent_tui_resize::task_agent_tui_resize;
use crate::features::tasks::agents::agent_tui::commands::task_agent_tui_write::task_agent_tui_write;
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
use crate::features::tasks::review::skills::install_predefined_skills_on_startup;
use crate::features::tasks::TaskManager;
use crate::features::theming::apply_startup_webview_window_css;
use crate::features::theming::apply_startup_window_background;
use crate::features::theming::on_page_load as theming_on_page_load;
use crate::features::time_tracking::commands::task_time_tracking_get::task_time_tracking_get;
use crate::features::time_tracking::commands::task_time_tracking_record::task_time_tracking_record;
use log::info;
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
use tauri::Manager;
use tauri_plugin_opener::OpenerExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = dotenvy::dotenv() {
        log::debug!("dotenv was not loaded: {error}");
    }
    if let Err(error) = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("illuc=debug,tauri=info"),
    )
    .format_timestamp_millis()
    .try_init()
    {
        log::debug!("logger was already initialized: {error}");
    }
    info!("starting illuc tauri app");

    tauri::Builder::default()
        .plugin(
            tauri::plugin::Builder::<tauri::Wry>::new("external-link-opener")
                .on_navigation(|app_webview, url| {
                    let scheme = url.scheme();
                    let host = url.host_str().unwrap_or_default();
                    let is_internal_host = host.eq_ignore_ascii_case("localhost")
                        || host.eq_ignore_ascii_case("tauri.localhost")
                        || host.ends_with(".localhost")
                        || host == "127.0.0.1"
                        || host == "::1";
                    let should_open_external = match scheme {
                        "http" | "https" => !is_internal_host,
                        "mailto" => true,
                        _ => false,
                    };
                    if should_open_external {
                        if let Err(error) = app_webview
                            .app_handle()
                            .opener()
                            .open_url(url.as_str(), None::<String>)
                        {
                            log::warn!("failed to open external url {}: {}", url, error);
                        }
                        return false;
                    }
                    true
                })
                .build(),
        )
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

            if let Err(error) = start_settings_theme_watcher(app.handle().clone()) {
                log::warn!("failed to start settings/theme watcher: {error}");
            }

            if let Err(error) = install_predefined_skills_on_startup() {
                log::warn!("failed to install predefined skills: {error}");
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
            task_agent_gui_compact,
            task_agent_gui_interrupt,
            task_agent_gui_models,
            task_agent_gui_new_chat,
            task_agent_gui_request_respond,
            task_agent_gui_rollback,
            task_agent_gui_send,
            task_agent_gui_usage,
            task_agent_tui_resize,
            task_agent_tui_write,
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
            settings_open_in_vscode,
            settings_theme_get
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
