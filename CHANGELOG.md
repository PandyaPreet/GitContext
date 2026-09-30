# Changelog

All notable changes to Git Context are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-09-30

### Added

- In-app SSH key generation: checks that the GitHub or GitLab username exists, creates an Ed25519 key with `ssh-keygen`, and links straight to the provider's SSH settings.
- Identity health check for missing keys, author/SSH conflicts and externally changed configuration, with **Reapply** for drift.
- **Verify active identity** for the global author, managed SSH configuration and key authentication.
- Quick switcher on `Cmd+Shift+G` / `Ctrl+Shift+G` that opens a compact picker without the main window.
- One-click profile activation, and profile colours shown in the sidebar, avatar and tray icon (plus the active profile name in the macOS menu bar).
- Groundwork for signed in-app updates. This release is not signed, so the app reports updates as not configured.

### Fixed

- SSH configuration validation on Windows, where OpenSSH could not read the staged config file.
- Release builds now fail loudly instead of finishing without attaching installers.

### Changed

- Update signing is optional in the release workflow; without signing keys, installers build as before.

## [0.1.0] - 2026-09-29

First public release.

### Added

- GitHub and GitLab profiles, including self-hosted hosts and custom SSH ports.
- Per-repository identity assignment: `user.name`, `user.email`, a managed SSH host alias and optional remote rewriting.
- Before/after preview of every change to SSH and Git configuration.
- A transaction journal with undo for each applied change.
- SSH identity verification against the provider.
- Automatic detection of Git, OpenSSH, existing SSH keys and hosts, the SSH agent and the GitHub CLI.
- A global default profile that can be switched from the system tray.
- A command palette (`⌘K` / `Ctrl K`).
- Launching a repository in VS Code, Terminal, iTerm, Windows Terminal, PowerShell, CMD or the file manager.
- Light and dark themes, launch at login, and a configurable startup view.
- Installers for macOS (Apple Silicon and Intel), Windows (x64 and ARM64) and Linux (x64 and ARM64).

[0.1.1]: https://github.com/PandyaPreet/GitContext/releases/tag/v0.1.1
[0.1.0]: https://github.com/PandyaPreet/GitContext/releases/tag/v0.1.0
