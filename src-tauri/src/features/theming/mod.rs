use tauri::Manager;
use tauri::utils::config::Color;

use crate::features::settings::{load_selected_syntax_theme_name, load_theme_settings};
use crate::features::settings::resolve_default_theme_name;

pub fn apply_startup_window_background(window: &tauri::WebviewWindow) {
    let default_theme_name = resolve_default_theme_name(Some(window.theme()));
    let app = window.app_handle();
    match load_theme_settings(app, &default_theme_name) {
        Ok(values) => {
            if let Some(bg) = values.get("surfaces.bg") {
                match bg.parse::<Color>() {
                    Ok(color) => {
                        if let Err(error) = window.set_background_color(Some(color)) {
                            log::warn!("failed to set window background color: {error}");
                        }
                    }
                    Err(error) => {
                        log::warn!(
                            "failed to parse surfaces.bg as a color ({}): {error}",
                            bg
                        );
                    }
                }
            } else {
                log::debug!("theme missing surfaces.bg; leaving default background");
            }
        }
        Err(_) => {
            log::debug!("failed to load theme settings early; leaving default background");
        }
    }
}

pub fn apply_startup_webview_window_css(window: &tauri::WebviewWindow) {
    let default_theme_name = resolve_default_theme_name(Some(window.theme()));
    let app = window.app_handle();
    let Some(js) = build_startup_webview_js(app, &default_theme_name) else {
        return;
    };
    if let Err(error) = window.eval(js) {
        log::warn!("failed to inject startup theme CSS into window webview: {error}");
    }
}

pub fn apply_startup_webview_css(webview: &tauri::Webview<tauri::Wry>) {
    let default_theme_name = resolve_default_theme_name(Some(webview.window().theme()));
    let window = webview.window();
    let app = window.app_handle();
    let Some(js) = build_startup_webview_js(app, &default_theme_name) else {
        return;
    };
    if let Err(error) = webview.eval(js) {
        log::warn!("failed to inject startup theme CSS into webview: {error}");
    }
}

pub fn on_page_load(
    webview: &tauri::Webview<tauri::Wry>,
    payload: &tauri::webview::PageLoadPayload<'_>,
) {
    if webview.window().label() != "main" {
        return;
    }

    // Inject theme variables/background at the earliest page load stage we can hook into.
    // Combined with `visible: false` in tauri.conf.json, this prevents any white flash and
    // ensures the HTML/body background is set before Angular renders.
    if matches!(payload.event(), tauri::webview::PageLoadEvent::Started) {
        apply_startup_webview_css(webview);
    }
}

fn build_startup_webview_js(app: &tauri::AppHandle, default_theme_name: &str) -> Option<String> {
    let values = load_theme_settings(app, default_theme_name).ok()?;
    let syntax_theme = load_selected_syntax_theme_name(app, default_theme_name)
        .unwrap_or_else(|_| "light".to_string());
    let bg_value = values
        .get("surfaces.bg")
        .map(String::as_str)
        .unwrap_or("#000000");

    let mut js =
        String::from("(function(){try{var root=document.documentElement;var style=root.style;");

    // Apply all flattened theme keys as CSS custom properties.
    for (key, value) in values.iter() {
        // Keep consistent with ThemeService mapping: --<dot-key with dots replaced by dashes>.
        let css_key = format!("--{}", key.replace('.', "-"));
        let css_key_js = serde_json::to_string(&css_key).ok()?;
        let value_js = serde_json::to_string(&value).ok()?;
        js.push_str("style.setProperty(");
        js.push_str(&css_key_js);
        js.push(',');
        js.push_str(&value_js);
        js.push_str(");");
    }

    // Ensure html/body background matches the selected theme immediately.
    if let Ok(bg) = serde_json::to_string(bg_value) {
        js.push_str("root.style.backgroundColor=");
        js.push_str(&bg);
        js.push_str(";");
        js.push_str("if(document.body){document.body.style.backgroundColor=");
        js.push_str(&bg);
        js.push_str(";}");
        js.push_str(
            "else{document.addEventListener('DOMContentLoaded',function(){if(document.body)document.body.style.backgroundColor=",
        );
        js.push_str(&bg);
        js.push_str(";},{once:true});}");
    }

    // Align syntax theme as early as possible (used by highlight.js overrides).
    if let Ok(syntax_js) = serde_json::to_string(&syntax_theme) {
        js.push_str("root.setAttribute('data-syntax-theme',");
        js.push_str(&syntax_js);
        js.push_str(");");
    }

    js.push_str("}catch(e){console.warn('startup theming injection failed',e);}})();");
    Some(js)
}
