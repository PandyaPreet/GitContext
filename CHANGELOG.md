# Changelog

All notable changes to Git Context are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

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

[0.1.0]: https://github.com/PandyaPreet/GitContext/releases/tag/v0.1.0
