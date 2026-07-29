//! syncparty — synchronised movie nights over Tailscale.
//!
//! Two halves in one binary. A host brings up a Syncplay server on its
//! tailnet address and hands out an invite; a guest opens that invite and is
//! dropped straight into the room. Both share the same dependency checks,
//! protocol code and settings, which is why they are not two apps.

pub mod core;
pub mod ipc;

use std::sync::Arc;

use tauri::Manager;

use crate::core::events::{AppEvent, EventBus};
use crate::core::invite::Invite;
use crate::ipc::{commands, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "syncparty=info".into()),
        )
        .init();

    let mut builder = tauri::Builder::default();

    // Must come first: it is what routes a second launch — which is how the
    // OS delivers a `syncparty://` link to an already-running app — back into
    // this process instead of starting a rival one.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            focus_main_window(app);
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = AppState::build(&handle)?;
            let bus = Arc::clone(&state.bus);
            app.manage(state);

            register_deep_links(&handle, bus);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::update_settings,
            commands::run_preflight,
            commands::install_dependency,
            commands::set_dependency_path,
            commands::start_hosting,
            commands::stop_hosting,
            commands::session_state,
            commands::decode_invite,
            commands::join_party,
            commands::join_hosted_party,
            commands::discord_status,
            commands::set_discord_webhook,
            commands::clear_discord_webhook,
            commands::test_discord_webhook,
        ])
        .run(tauri::generate_context!())
        .expect("syncparty failed to start");
}

/// Listens for `syncparty://` links and republishes valid ones as events.
///
/// Both entry points are covered: a link that started the app cold, and one
/// that arrives while it is already open.
fn register_deep_links(app: &tauri::AppHandle, bus: Arc<dyn EventBus>) {
    use tauri_plugin_deep_link::DeepLinkExt;

    if let Ok(Some(urls)) = app.deep_link().get_current() {
        publish_first_invite(&urls, bus.as_ref());
    }

    let on_open = Arc::clone(&bus);
    app.deep_link().on_open_url(move |event| {
        publish_first_invite(event.urls().as_slice(), on_open.as_ref());
    });
}

/// Publishes the first URL that actually parses as an invite.
///
/// A malformed link is dropped rather than surfaced: these arrive from
/// outside the app, and a stray `syncparty://` from anywhere should not be
/// able to put an error in front of the user.
fn publish_first_invite(urls: &[url::Url], bus: &dyn EventBus) {
    let invite = urls
        .iter()
        .find_map(|url| Invite::decode(url.as_str()).ok());

    if let Some(invite) = invite {
        bus.publish(AppEvent::InviteReceived { invite });
    }
}

fn focus_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
