use crate::model::Result;
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub fn launch(kind: &str, path: &Path) -> Result<()> {
    if !path.is_absolute() || !path.is_dir() {
        return Err("Repository directory is unavailable".into());
    }
    if path.to_string_lossy().chars().any(char::is_control) {
        return Err("Unsupported control character in repository path".into());
    }
    if kind == "vscode" {
        return spawn(Command::new("code").arg("--new-window").arg("--").arg(path));
    }
    launch_native(kind, path)
}
pub fn open_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("Refusing to open an unsafe URL".into());
    }
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let mut command = Command::new("xdg-open");
    spawn(command.arg(url))
}
fn spawn(command: &mut Command) -> Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|_| "Launcher unavailable. Check that the selected application is installed and available in PATH.")?;
    // Reap the launcher without blocking the UI or retaining a zombie process.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_native(kind: &str, path: &Path) -> Result<()> {
    let script = match kind {
        "terminal" => "on run argv\ntell application \"Terminal\"\nactivate\ndo script (\"cd -- \" & quoted form of item 1 of argv)\nend tell\nend run",
        "iterm" => "on run argv\ntell application \"iTerm\"\nactivate\nset newWindow to (create window with default profile)\ntell current session of newWindow\nwrite text (\"cd -- \" & quoted form of item 1 of argv)\nend tell\nend tell\nend run",
        "folder" => return spawn(Command::new("open").arg(path)),
        _ => return Err("This launcher is not available on macOS".into()),
    };
    let out = crate::process::run(
        "osascript",
        &[
            "-e",
            script,
            path.to_str().ok_or("Invalid repository path")?,
        ],
        None,
    )?;
    if out.code == 0 {
        Ok(())
    } else {
        Err("macOS could not open the terminal. Check application installation and Automation permission.".into())
    }
}

#[cfg(target_os = "windows")]
fn launch_native(kind: &str, path: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt;
    if kind == "terminal" && path.to_string_lossy().contains(';') {
        return Err("Windows Terminal treats semicolons as command separators. Use PowerShell or CMD for this repository path.".into());
    }
    match kind {
        "terminal" => spawn(
            Command::new("wt.exe")
                .arg("-w")
                .arg("new")
                .arg("new-tab")
                .arg("--startingDirectory")
                .arg(path),
        ),
        "powershell" => {
            let literal = path.to_string_lossy().replace('\'', "''");
            spawn(
                Command::new("powershell.exe")
                    .creation_flags(0x00000010)
                    .args([
                        "-NoLogo",
                        "-NoProfile",
                        "-NoExit",
                        "-Command",
                        &format!("Set-Location -LiteralPath '{literal}'"),
                    ]),
            )
        }
        "cmd" => spawn(
            Command::new("cmd.exe")
                .creation_flags(0x00000010)
                .arg("/D")
                .arg("/K")
                .current_dir(path),
        ),
        "folder" => spawn(Command::new("explorer.exe").arg(path)),
        _ => Err("This launcher is not available on Windows".into()),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_native(kind: &str, path: &Path) -> Result<()> {
    match kind {
        "terminal" => spawn(Command::new("x-terminal-emulator").current_dir(path)),
        "folder" => spawn(Command::new("xdg-open").arg(path)),
        _ => Err("Unsupported launcher on this platform".into()),
    }
}

/// macOS Dock/application-menu Quit keeps the tray service alive. Explicit
/// app.exit/app.restart requests carry a code and must be allowed through.
pub fn handle_background_exit(app: &tauri::AppHandle, event: &tauri::RunEvent) {
    #[cfg(target_os = "macos")]
    if let tauri::RunEvent::ExitRequested {
        code: None, api, ..
    } = event
    {
        use tauri::Manager;
        // Never trap the user in a background process without a usable tray.
        if app.tray_by_id("context").is_some() {
            api.prevent_exit();
            for window in app.webview_windows().values() {
                let _ = window.hide();
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, event);
}
