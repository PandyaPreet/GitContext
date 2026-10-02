pub mod bridge;
mod detection;
mod git;
mod global;
mod model;
mod platform;
mod process;
mod provider;
mod service;
mod ssh;
mod storage;

use model::*;
use service::Service;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
struct ShortcutStatus(Option<String>);
#[tauri::command]
fn shortcut_status(state: State<'_, ShortcutStatus>) -> Option<String> {
    state.0.clone()
}

type Shared = Arc<Mutex<Service>>;

async fn work<T: Send + 'static>(
    state: &State<'_, Shared>,
    operation: impl FnOnce(&mut Service) -> Result<T> + Send + 'static,
) -> Result<T> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut service = shared.lock().map_err(|_| "Application state unavailable")?;
        let _lock = storage::ConfigLock::acquire(&service.root.join("state.json"))?;
        bridge::synchronize(&mut service)?;
        operation(&mut service)
    })
    .await
    .map_err(|_| "Background operation failed")?
}

#[tauri::command]
async fn snapshot(state: State<'_, Shared>) -> Result<AppData> {
    work(&state, |s| Ok(s.data.clone())).await
}
#[tauri::command]
async fn configuration_health(state: State<'_, Shared>) -> Result<HealthReport> {
    work(&state, |s| Ok(s.health())).await
}
#[tauri::command]
async fn verify_active_identity(state: State<'_, Shared>) -> Result<Verification> {
    work(&state, |s| {
        let report = s.health();
        if report.checks.iter().any(|c| !c.ok) {
            return Err(
                "Configuration needs attention. Run the health check before verifying.".into(),
            );
        }
        let id = report.profile_id.ok_or("No active profile")?;
        let result = ssh::verify(s.profile(&id)?)?;
        // The same service/config lock covers checks and authentication.
        if s.health().checks.iter().any(|c| !c.ok) {
            return Err("Configuration changed during verification. Check again.".into());
        }
        Ok(result)
    })
    .await
}
#[tauri::command]
async fn set_profile_color(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile_id: String,
    color: String,
) -> Result<AppData> {
    let data = work(&state, move |s| s.set_color(&profile_id, &color)).await?;
    let _ = refresh_tray(&app, &data);
    let _ = app.emit("context-changed", &data);
    Ok(data)
}
#[tauri::command]
fn dismiss_switcher(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("switcher") {
        let _ = window.hide();
    }
}
#[tauri::command]
fn updater_ready(app: tauri::AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|v| v.get("pubkey"))
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.is_empty())
}
#[tauri::command]
async fn detect_environment() -> Result<Detection> {
    tauri::async_runtime::spawn_blocking(detection::detect)
        .await
        .map_err(|_| "Detection failed")?
}
#[tauri::command]
async fn import_key(public_path: String) -> Result<SshKey> {
    tauri::async_runtime::spawn_blocking(move || {
        detection::inspect_key(&detection::expand(&public_path)?, "")
    })
    .await
    .map_err(|_| "Key import failed")?
}
#[tauri::command]
async fn generate_key(name: String, comment: String) -> Result<SshKey> {
    tauri::async_runtime::spawn_blocking(move || detection::generate_key(&name, &comment))
        .await
        .map_err(|_| "Key generation failed")?
}
#[tauri::command]
async fn check_account(
    provider: provider::GitProvider,
    host: String,
    username: String,
) -> Result<AccountCheck> {
    tauri::async_runtime::spawn_blocking(move || {
        provider::check_account(provider, &host, &username)
    })
    .await
    .map_err(|_| "Account lookup failed")?
}
#[tauri::command]
async fn open_key_settings(provider: provider::GitProvider, host: String) -> Result<()> {
    let url = provider.ssh_keys_url(&host)?;
    tauri::async_runtime::spawn_blocking(move || platform::open_url(&url))
        .await
        .map_err(|_| "Could not open the browser")?
}
#[tauri::command]
async fn create_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile: Profile,
) -> Result<AppData> {
    let data = work(&state, move |s| s.create_profile(profile)).await?;
    refresh_tray(&app, &data).map_err(|_| "Profile saved, but tray refresh failed")?;
    Ok(data)
}
#[tauri::command]
async fn update_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile: Profile,
) -> Result<AppData> {
    let data = work(&state, move |s| s.update_profile(profile)).await?;
    refresh_tray(&app, &data).map_err(|_| "Profile saved, but tray refresh failed")?;
    Ok(data)
}
#[tauri::command]
async fn rename_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile_id: String,
    name: String,
) -> Result<AppData> {
    let data = work(&state, move |s| s.rename_profile(&profile_id, &name)).await?;
    refresh_tray(&app, &data).map_err(|_| "Profile renamed, but tray refresh failed")?;
    Ok(data)
}
#[tauri::command]
async fn remove_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile_id: String,
) -> Result<AppData> {
    let data = work(&state, move |s| s.remove_profile(&profile_id)).await?;
    refresh_tray(&app, &data).map_err(|_| "Profile removed, but tray refresh failed")?;
    Ok(data)
}
#[tauri::command]
async fn update_repository(
    state: State<'_, Shared>,
    repository_id: String,
    name: String,
    path: String,
) -> Result<AppData> {
    work(&state, move |s| {
        s.update_repository(&repository_id, &name, &path)
    })
    .await
}
#[tauri::command]
async fn remove_repository(state: State<'_, Shared>, repository_id: String) -> Result<AppData> {
    work(&state, move |s| s.remove_repository(&repository_id)).await
}
#[tauri::command]
async fn register_key(state: State<'_, Shared>, public_path: String) -> Result<AppData> {
    work(&state, move |s| s.register_key(&public_path)).await
}
#[tauri::command]
async fn remove_key_reference(state: State<'_, Shared>, public_path: String) -> Result<AppData> {
    work(&state, move |s| s.remove_key_reference(&public_path)).await
}
#[tauri::command]
async fn choose_public_key(app: tauri::AppHandle) -> Result<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app
            .dialog()
            .file()
            .set_title("Choose an SSH public key")
            .add_filter("OpenSSH public key", &["pub"]);
        if let Some(window) = app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
        dialog
            .blocking_pick_file()
            .map(|file| {
                file.into_path()
                    .map_err(|_| "Choose a local .pub file".to_string())?
                    .into_os_string()
                    .into_string()
                    .map_err(|_| "Key path must be UTF-8".to_string())
            })
            .transpose()
    })
    .await
    .map_err(|_| "Could not open the file picker")?
}
#[tauri::command]
async fn open_repository_folder(state: State<'_, Shared>, repository_id: String) -> Result<()> {
    work(&state, move |s| {
        platform::launch(
            "folder",
            std::path::Path::new(&s.repository(&repository_id)?.path),
        )
    })
    .await
}
#[tauri::command]
async fn select_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile_id: String,
) -> Result<AppData> {
    let data = work(&state, move |s| s.select_profile(&profile_id)).await?;
    refresh_tray(&app, &data).map_err(|_| "Context saved, but tray refresh failed")?;
    Ok(data)
}
#[tauri::command]
async fn choose_repository_folder(app: tauri::AppHandle) -> Result<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file().set_title("Choose a Git repository");
        if let Some(window) = app.get_webview_window("main") {
            dialog = dialog.set_parent(&window);
        }
        dialog
            .blocking_pick_folder()
            .map(|folder| {
                folder
                    .into_path()
                    .map_err(|_| "Choose a local folder".to_string())?
                    .into_os_string()
                    .into_string()
                    .map_err(|_| "Folder path must be UTF-8".to_string())
            })
            .transpose()
    })
    .await
    .map_err(|_| "Could not open the folder picker")?
}
#[tauri::command]
async fn register_repository(state: State<'_, Shared>, path: String) -> Result<AppData> {
    work(&state, move |s| s.register_repository(&path)).await
}
#[tauri::command]
async fn repository_status(state: State<'_, Shared>, repository_id: String) -> Result<RepoStatus> {
    work(&state, move |s| {
        let repo = s.repository(&repository_id)?;
        let mut status = git::inspect(
            repo,
            repo.profile_id.as_ref().and_then(|id| s.profile(id).ok()),
            &s.data.profiles,
        )?;
        if let Some(id) = &repo.profile_id {
            status.identity_matches &= s.validate_ssh(id).is_ok();
        }
        Ok(status)
    })
    .await
}
#[tauri::command]
async fn plan_assignment(
    state: State<'_, Shared>,
    repository_id: String,
    profile_id: String,
    rewrite_remote: bool,
) -> Result<PlanView> {
    work(&state, move |s| {
        s.plan_assignment(&repository_id, &profile_id, rewrite_remote)
    })
    .await
}
#[tauri::command]
async fn activate_profile(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    profile_id: String,
) -> Result<AppData> {
    let data = work(&state, move |s| {
        let plan = s.plan_activation(&profile_id)?;
        s.apply(&plan.id)?;
        Ok(s.data.clone())
    })
    .await?;
    let _ = refresh_tray(&app, &data);
    let _ = app.emit("context-changed", &data);
    Ok(data)
}
#[tauri::command]
async fn plan_activation(state: State<'_, Shared>, profile_id: String) -> Result<PlanView> {
    work(&state, move |s| s.plan_activation(&profile_id)).await
}
#[tauri::command]
async fn plan_global_profile(state: State<'_, Shared>, profile_id: String) -> Result<PlanView> {
    work(&state, move |s| s.plan_global(&profile_id)).await
}
#[tauri::command]
async fn apply_assignment(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    plan_id: String,
) -> Result<String> {
    let (id, data) = work(&state, move |s| {
        let id = s.apply(&plan_id)?;
        Ok((id, s.data.clone()))
    })
    .await?;
    let _ = refresh_tray(&app, &data);
    Ok(id)
}
#[tauri::command]
async fn list_transactions(state: State<'_, Shared>) -> Result<Vec<(String, String)>> {
    work(&state, |s| s.journals()).await
}
#[tauri::command]
async fn undo_assignment(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
    transaction_id: String,
) -> Result<AppData> {
    let data = work(&state, move |s| s.undo(&transaction_id)).await?;
    let _ = refresh_tray(&app, &data);
    Ok(data)
}
#[tauri::command]
async fn verify_profile(state: State<'_, Shared>, profile_id: String) -> Result<Verification> {
    let profile = work(&state, move |s| Ok(s.profile(&profile_id)?.clone())).await?;
    tauri::async_runtime::spawn_blocking(move || ssh::verify(&profile))
        .await
        .map_err(|_| "Verification failed")?
}
#[tauri::command]
async fn launch_terminal(
    state: State<'_, Shared>,
    repository_id: String,
    profile_id: String,
    kind: String,
) -> Result<()> {
    work(&state, move |s| {
        let repo = s.repository(&repository_id)?;
        if repo.profile_id.as_deref() != Some(&profile_id) {
            return Err("Apply the selected profile to this repository before launching".into());
        }
        if !git::inspect(repo, Some(s.profile(&profile_id)?), &s.data.profiles)?.identity_matches {
            return Err(
                "Repository identity changed. Review and reapply its profile first.".into(),
            );
        }
        s.validate_ssh(&profile_id)?;
        platform::launch(&kind, std::path::Path::new(&repo.path))
    })
    .await
}
#[tauri::command]
async fn save_settings(state: State<'_, Shared>, settings: Settings) -> Result<AppData> {
    work(&state, move |s| {
        if !["light", "dark"].contains(&settings.theme.as_str())
            || !["dashboard", "tray", "selector"].contains(&settings.startup_view.as_str())
        {
            return Err("Invalid settings".into());
        }
        let mut next = s.data.clone();
        next.settings = settings;
        s.commit(next)
    })
    .await
}

fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
fn tray_image(color: &str) -> tauri::image::Image<'static> {
    let rgb = match color {
        "blue" => [100, 165, 255],
        "violet" => [179, 145, 255],
        "amber" => [244, 183, 74],
        _ => [119, 221, 177],
    };
    let mut pixels = vec![0; 22 * 22 * 4];
    for y in 3..19 {
        for x in 3..19 {
            if x < 6 || !(6..=15).contains(&y) || (x > 12 && y > 9) {
                pixels[(y * 22 + x) * 4..(y * 22 + x) * 4 + 4]
                    .copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
    tauri::image::Image::new_owned(pixels, 22, 22)
}
fn show_switcher(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("switcher") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("switcher-opened", ());
    }
}
fn refresh_tray(app: &tauri::AppHandle, data: &AppData) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    let menu = Menu::new(app)?;
    for profile in &data.profiles {
        let active =
            data.single_profile_mode && data.global_profile_id.as_deref() == Some(&profile.id);
        menu.append(&MenuItem::with_id(
            app,
            format!("profile:{}", profile.id),
            format!(
                "{} {} · {}",
                if active { "✓" } else { "○" },
                profile.name,
                profile.provider.label()
            ),
            true,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        "Open Git Context",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "quit",
        "Quit Git Context Completely",
        true,
        None::<&str>,
    )?)?;
    if let Some(tray) = app.tray_by_id("context") {
        tray.set_menu(Some(menu))?;
        let name = data
            .profiles
            .iter()
            .find(|p| data.single_profile_mode && Some(&p.id) == data.global_profile_id.as_ref())
            .map(|p| p.name.as_str())
            .unwrap_or("No profile");
        let color = data
            .profiles
            .iter()
            .find(|p| data.single_profile_mode && Some(&p.id) == data.global_profile_id.as_ref())
            .map(|p| p.color.as_str())
            .unwrap_or("mint");
        tray.set_icon(Some(tray_image(color)))?;
        #[cfg(target_os = "macos")]
        tray.set_title(Some(name.chars().take(18).collect::<String>()))?;
        tray.set_tooltip(Some(format!("Git Context · {name}")))?;
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let service = Service::new(root).map_err(std::io::Error::other)?;
            let data = service.data.clone();
            app.manage(Arc::new(Mutex::new(service)));
            tauri::WebviewWindowBuilder::new(
                app,
                "switcher",
                tauri::WebviewUrl::App("index.html?switcher".into()),
            )
            .title("Switch Git profile")
            .inner_size(480.0, 420.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build()?;
            // A conflict must not prevent the desktop app from launching.
            let shortcut =
                app.global_shortcut()
                    .on_shortcut("CommandOrControl+Shift+G", |app, _, event| {
                        if event.state == ShortcutState::Pressed {
                            show_switcher(app);
                        }
                    });
            app.manage(ShortcutStatus(shortcut.err().map(|e| e.to_string())));
            tauri::tray::TrayIconBuilder::with_id("context")
                .icon(tray_image("mint"))
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    if id == "quit" {
                        app.exit(0);
                    } else if id == "open" {
                        show(app);
                    } else if let Some(profile_id) = id.strip_prefix("profile:") {
                        let app = app.clone();
                        let profile_id = profile_id.to_string();
                        // Run natively: switching must not show/focus the window
                        // or depend on a visible, responsive webview.
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<Shared>();
                            if let Err(error) =
                                activate_profile(app.clone(), state, profile_id).await
                            {
                                let _ = app.emit("context-error", error);
                                if let Some(tray) = app.tray_by_id("context") {
                                    let _ = tray.set_tooltip(Some(
                                        "Git Context · Switch failed. Open the app for details.",
                                    ));
                                }
                            }
                        });
                    }
                })
                .build(app)?;
            refresh_tray(app.handle(), &data)?;
            if std::env::args().any(|a| a == "--autostart") && data.settings.startup_view == "tray"
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.hide()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "switcher" && matches!(event, tauri::WindowEvent::Focused(false)) {
                let _ = window.hide();
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            configuration_health,
            verify_active_identity,
            set_profile_color,
            dismiss_switcher,
            updater_ready,
            shortcut_status,
            detect_environment,
            import_key,
            generate_key,
            check_account,
            open_key_settings,
            create_profile,
            rename_profile,
            remove_profile,
            update_repository,
            remove_repository,
            register_key,
            remove_key_reference,
            update_profile,
            choose_public_key,
            open_repository_folder,
            select_profile,
            register_repository,
            choose_repository_folder,
            repository_status,
            plan_assignment,
            apply_assignment,
            plan_global_profile,
            plan_activation,
            activate_profile,
            list_transactions,
            undo_assignment,
            verify_profile,
            launch_terminal,
            save_settings
        ])
        .build(tauri::generate_context!())
        .expect("Unable to start Git Context")
        .run(|_app, _event| {
            platform::handle_background_exit(_app, &_event);
            // Dock/Finder reopen requests do not launch a second instance.
            // Restore the existing window that CloseRequested hides for tray use.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show(_app);
            }
        });
}
