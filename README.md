<div align="center">

<img src="src-tauri/icons/128x128@2x.png" alt="Git Context" width="112" height="112" />

# Git Context

**Use the right Git identity in every repository, automatically.**

A local desktop app for developers with more than one GitHub or GitLab account.<br/>
Git Context sets the right name, email and SSH key for each repository, so you stop committing as the wrong person.

[![Latest release](https://img.shields.io/github/v/release/PandyaPreet/GitContext?display_name=tag&sort=semver&color=77ddb1)](https://github.com/PandyaPreet/GitContext/releases/latest)
[![Desktop checks](https://github.com/PandyaPreet/GitContext/actions/workflows/ci.yml/badge.svg)](https://github.com/PandyaPreet/GitContext/actions/workflows/ci.yml)
![Platforms](https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-informational)
![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24c8db?logo=tauri&logoColor=white)
![Privacy](https://img.shields.io/badge/privacy-no%20data%20collected-2ea44f)

[**Download**](#-download) · [Features](#-features) · [Security & privacy](#-security--privacy) · [How it works](#-how-it-works) · [FAQ](#-faq) · [Contributing](#-contributing)

🔒 **Runs entirely on your computer. No account, no cloud, no tracking. Your keys and config never leave your machine.**

</div>

<!-- Add a product screenshot here, e.g. docs/screenshot.png:
<p align="center"><img src="docs/screenshot.png" alt="Git Context dashboard" width="860" /></p>
-->

---

## The problem

You have a work account and a personal one. Maybe a few client accounts too. Then one day you:

- push a work commit authored by `me@personal.dev`,
- get `Permission denied (publickey)` because SSH offered the wrong key,
- or hand-edit `~/.ssh/config` for the fifth time this month.

**Git Context fixes this.** You create a profile once for each identity, assign a profile to each repository, and the app writes the correct Git and SSH configuration for you. You can preview every change first and undo it afterwards.

## ✨ Features

| | |
| --- | --- |
| 👤 **Profiles for every identity** | GitHub and GitLab, including self-hosted hosts and custom SSH ports. Each profile has a name, email, username, SSH key and color. |
| 🔑 **Generate SSH keys in-app** | Checks that your GitHub or GitLab username exists, then creates an Ed25519 key in one click, copy the public key, and jump straight to your GitHub or GitLab SSH settings to paste it. |
| 📁 **Per-repository assignment** | Sets `user.name`, `user.email` and a dedicated SSH host alias for a repository, and can rewrite its remote to match. |
| 🔍 **Preview before apply** | Every change to `~/.ssh/config`, `~/.gitconfig` or a repository's config appears as a before/after diff first. Nothing is written until you confirm. |
| ↩️ **Undo** | Every applied change is journaled. Roll it back from **Settings → Configuration history**. |
| 🔐 **SSH verification** | Confirms that a key actually authenticates as the account you expect, not just any account. |
| 🌐 **Global default profile** | Switch your machine-wide identity in one click from the system tray. |
| 🧭 **Environment detection** | Finds your Git and OpenSSH versions, existing keys, SSH hosts, the SSH agent and the GitHub CLI. |
| ⌨️ **Command palette** | Press `⌘K` / `Ctrl K` to activate profiles and run actions from the keyboard. |
| 🚀 **Launch in context** | Open a verified repository in VS Code, Terminal, iTerm, Windows Terminal, PowerShell, CMD or your file manager. |
| 🌗 **Made for daily use** | Light and dark themes, launch at login, starts in the tray, single-instance. |

## 🔒 Security & privacy

Git Context works with your SSH keys and Git identity, so it's built to be trustworthy from the ground up.

### We don't collect your data

- **No account or sign-up.** Download and use it. There's nothing to register.
- **No servers, no cloud.** There's no Git Context backend. We don't run servers that store your information.
- **No telemetry, analytics or tracking.** The app contains no analytics SDK and never reports usage, crashes or device information.
- **No ads, no third-party trackers.** None.
- **Open source.** Every line of code is in this repository, so you can check these claims yourself.

### Your SSH keys stay yours

- Git Context **never copies, uploads or modifies your private keys**.
- New keys you generate are created by your system's own `ssh-keygen` in `~/.ssh`, and existing files are never overwritten.
- It stores only the **file path** to your keys, such as `~/.ssh/id_ed25519_work`.
- Authentication is done by your system's own OpenSSH, exactly as when you run `git push`.

### What stays on your computer

To work and support undo, Git Context saves a small amount of data **locally, on your own machine only**:

| Data | Why |
| --- | --- |
| Profiles: display name, Git name and email, username, host, SSH key paths | So you can switch identities |
| Paths of the repositories you add | So it knows which repository uses which profile |
| Backups of config files it changed (`~/.ssh/config`, `~/.gitconfig`, repository config) | So every change can be undone |
| App settings (theme, startup view) | Your preferences |

It's stored in your OS's standard app-data folder (see [How it works](#-how-it-works)). On macOS and Linux, files are saved with owner-only permissions. Uninstall the app and delete that folder, and everything is gone.

### When does it use the network?

Git Context has no servers of its own. It talks only to your Git provider (GitHub, GitLab or your self-hosted GitLab), and only for these checks:

- **Username check**: while you create a profile, the app asks the provider's public API whether the username exists, using your system's `curl`. Only the username is sent: no email, keys or configuration.

- **Verify profile** runs `ssh -T git@<your-host>` so GitHub or GitLab can confirm which account a key belongs to, the same check their docs recommend.
- **Environment detection**: if you have the GitHub CLI (`gh`) installed, the app runs `gh auth status` to suggest the usernames you're signed in with. If you don't have `gh`, nothing happens.

### Built to be safe

- **Locked-down app.** The interface has no shell, filesystem or network permissions and runs under a strict Content Security Policy. Every action goes through a small, validated Rust backend.
- **No network listener.** The app doesn't open ports or run a local web server.
- **You approve every change.** Every edit is shown as a before/after preview, and nothing is written until you click **Apply**.
- **Undo anything.** Every applied change is journaled and can be rolled back.
- **Careful with your config.** Git Context edits only its own clearly marked blocks. It refuses to change setups it can't reason about safely (`Include`/`Match` rules, `core.sshCommand`, `url.*.insteadOf`) and asks you to review them instead.
- **Crash-safe writes.** Files are written atomically and under a lock, so an interrupted write can't leave a half-written config.
- **Strict host checking.** Managed SSH aliases use `IdentitiesOnly yes` and `StrictHostKeyChecking yes`, so only the chosen key is offered and only to a known host.

### Reporting a vulnerability

Found a security issue? Please report it privately through [GitHub Security Advisories](https://github.com/PandyaPreet/GitContext/security/advisories/new) instead of opening a public issue. We'll respond as quickly as we can.

## 📦 Download

Get the latest version from the **[Releases page](https://github.com/PandyaPreet/GitContext/releases/latest)**.

| Platform | Architecture | Installer |
| --- | --- | --- |
| **macOS** 10.15+ | Apple Silicon (M1–M4) | `Git.Context_<version>_aarch64.dmg` |
| | Intel | `Git.Context_<version>_x64.dmg` |
| **Windows** 10/11 | x64 | `Git.Context_<version>_x64-setup.exe` or `_x64_en-US.msi` |
| | ARM64 | `Git.Context_<version>_arm64-setup.exe` |
| **Linux** | x64 | `.AppImage` · `.deb` (Debian/Ubuntu) · `.rpm` (Fedora/RHEL/openSUSE) |
| | ARM64 | `.AppImage` · `.deb` · `.rpm` |

**Requirements:** [Git](https://git-scm.com/downloads) and OpenSSH on your `PATH`. OpenSSH ships with macOS, Windows 10+ and most Linux distributions.

<details>
<summary><b>macOS: "Git Context can't be opened" or "is damaged"</b></summary>

Release builds aren't notarized with a paid Apple Developer certificate yet. Open the app once from Finder with a right-click → **Open** → **Open**. If macOS reports the app as damaged, remove the download quarantine flag:

```bash
xattr -dr com.apple.quarantine "/Applications/Git Context.app"
```
</details>

<details>
<summary><b>Windows: "Windows protected your PC"</b></summary>

SmartScreen warns about apps without an established code-signing reputation. Click **More info** → **Run anyway**.
</details>

<details>
<summary><b>Linux: installing each package type</b></summary>

```bash
# Debian / Ubuntu
sudo apt install ./<downloaded-file>.deb

# Fedora / RHEL / openSUSE
sudo dnf install ./<downloaded-file>.rpm

# Any distribution
chmod +x ./<downloaded-file>.AppImage
./<downloaded-file>.AppImage
```

The tray icon needs an AppIndicator-compatible desktop. On GNOME, install the *AppIndicator and KStatusNotifierItem Support* extension.
</details>

## 🚀 Quick start

1. **Launch Git Context.** It detects your Git setup and any SSH keys in `~/.ssh`.
2. **Create a profile** for each identity, for example *Work* and *Personal*. Pick an existing SSH key, or click **Generate new key** and add it to your account with the **Copy** and **Open SSH settings** buttons.
3. **Verify** each profile. Git Context asks GitHub or GitLab which account the key belongs to.
4. **Add a repository**, choose a profile, review the preview and click **Apply**.
5. Commit and push as usual. The right identity is used every time.

> 💡 **New to SSH keys?** Click **Generate new key** while creating a profile. Git Context creates the key, lets you copy it, and opens your GitHub or GitLab SSH settings so you can paste it.

## ⚙️ How it works

For each profile, Git Context adds a clearly marked block to `~/.ssh/config`:

```sshconfig
# Git Context begin <profile-id>
Host github-work
    HostName github.com
    User git
    IdentityFile "~/.ssh/id_ed25519_work"
    IdentitiesOnly yes
    StrictHostKeyChecking yes
Host *
# Git Context end <profile-id>
```

When you assign that profile to a repository, it:

1. sets the repository's local `user.name` and `user.email`,
2. optionally rewrites the remote from `git@github.com:acme/api.git` to `git@github-work:acme/api.git`, so SSH always offers the right key,
3. records a transaction you can undo later.

The **global default profile** writes the same settings to `~/.gitconfig` and a `Host github.com` block, and never touches individual repositories.

App state is stored locally in:

| OS | Location |
| --- | --- |
| macOS | `~/Library/Application Support/dev.gitcontext.desktop` |
| Windows | `%APPDATA%\dev.gitcontext.desktop` |
| Linux | `~/.local/share/dev.gitcontext.desktop` |

## ❓ FAQ

<details>
<summary><b>Will it break my existing SSH or Git configuration?</b></summary>

No. Git Context only edits its own marked blocks, shows every change before applying it, and refuses to proceed when it finds rules it can't reason about safely (`Include`, `Match`, `core.sshCommand`, `url.*.insteadOf`). Every applied change can be undone.
</details>

<details>
<summary><b>Does it upload or store my private keys?</b></summary>

No. It only stores the *path* to keys you already have. Nothing is sent anywhere except a public username lookup and the standard `ssh -T` handshake, both only to your own Git provider.
</details>

<details>
<summary><b>Do you collect any of my data?</b></summary>

No. There's no account, server, analytics or telemetry. The little data the app needs, like your profiles and undo backups, is saved only on your own computer. See [Security & privacy](#-security--privacy).
</details>

<details>
<summary><b>How do I remove everything?</b></summary>

Undo any applied changes from **Settings → Configuration history**, uninstall the app, then delete its app-data folder (listed under [How it works](#-how-it-works)).
</details>

<details>
<summary><b>Does it support GitLab or self-hosted instances?</b></summary>

Yes. GitHub, GitLab.com and self-hosted GitLab (or GitHub Enterprise) hosts with any SSH port are supported.
</details>

<details>
<summary><b>Does it work with HTTPS remotes?</b></summary>

Git Context manages SSH-based identities. It can set name and email on any repository, but key selection relies on SSH remotes. It can rewrite a remote to the matching SSH alias for you.
</details>

## 🛠️ Development

**Prerequisites:** [Node.js 24+](https://nodejs.org/), [Rust 1.85+](https://rustup.rs/), and the [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
git clone https://github.com/PandyaPreet/GitContext.git
cd GitContext
npm install

npm run desktop         # run the app with hot reload
npm test                # frontend tests (Vitest)
npm run desktop:build   # build installers for your current OS
```

Backend checks:

```bash
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --lib
```

### Tech stack

- **Desktop shell:** [Tauri 2](https://tauri.app) (Rust)
- **UI:** React 19, TypeScript, Vite, Tailwind CSS 4, Radix UI, cmdk, Lucide icons
- **Testing:** Vitest and Testing Library for the UI, `cargo test` for the backend

### Project structure

```
src/                     React frontend
  components/            screens, dialogs and UI primitives
  hooks/ lib/            state, typed IPC API, provider helpers
src-tauri/src/
  lib.rs                 Tauri commands, tray and app lifecycle
  service.rs             plans, apply and undo orchestration
  ssh.rs  global.rs      SSH config parsing, merging and verification
  git.rs  provider.rs    repository inspection and GitHub/GitLab rules
  storage.rs             atomic writes, locking and transaction journal
  detection.rs           environment and SSH key discovery
  platform.rs            terminal, editor and folder launchers
  bridge.rs              local JSON-lines helper protocol
```

## 🚢 Releasing

Releases are built by [`.github/workflows/release.yml`](.github/workflows/release.yml) for macOS (arm64, x64), Windows (x64, arm64) and Linux (x64, arm64).

1. Bump `version` in `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`.
2. Update [`CHANGELOG.md`](CHANGELOG.md) and [`.github/RELEASE_NOTES.md`](.github/RELEASE_NOTES.md).
3. Tag and push:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
4. When the workflow finishes, review the draft on the Releases page and click **Publish**.

## 🤝 Contributing

Bug reports, ideas and pull requests are welcome.

1. [Open an issue](https://github.com/PandyaPreet/GitContext/issues) describing the bug or feature.
2. Fork the repository and create a branch.
3. Make sure `npm test` and the Cargo checks above pass.
4. Open a pull request.

## 💚 Support

If Git Context saves you from one wrong-author commit, please ⭐ **star the repo**. It helps other developers find it.

<div align="center">
<sub>Built with Rust, React and Tauri.</sub>
</div>

### Profile awareness and quick switching

- Choose Mint, Blue, Violet or Amber when adding a profile, or choose a colour from a profile's **…** menu. The sidebar, profile avatar and tray icon use that colour. On macOS, the menu bar also shows the active profile name.
- Press **Command+Shift+G** on macOS or **Ctrl+Shift+G** on Windows/Linux to open the compact picker. Type to filter, use arrow keys and Enter to activate, or Escape to dismiss. The main window stays closed. Shortcut conflicts are shown under **Quick switch & app updates**.
- **Verify active identity** checks the global author and managed SSH configuration, then tests the selected key against GitHub/GitLab. **Active** means applied; **Verified** means a successful authentication observation. It does not prove access to every repository, and HTTPS credentials remain independent.
- Open **Health check** to inspect missing keys, author/SSH conflicts and externally changed configuration. Checks also run on focus and every minute while the webview is running. Use **Reapply** for settings drift; conflicting managed edits require review or restoration through Configuration history. Health checks never rewrite configuration.

### Enable signed automatic updates before the next release

The app checks for updates shortly after launch and daily, displays release notes, and installs only when the user clicks **Install update**. A local build without a signing public key clearly reports that updates are not configured.

1. Generate a dedicated updater signing key **outside this repository**, and keep a secure backup:
   ```bash
   npm run tauri -- signer generate -w "$HOME/.tauri/git-context-updater.key"
   ```
2. In GitHub → Settings → Secrets and variables → Actions, add:
   - Repository variable `TAURI_UPDATER_PUBLIC_KEY`: contents of the generated `.key.pub` file.
   - Repository secret `TAURI_SIGNING_PRIVATE_KEY`: contents of the private `.key` file.
   - Repository secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: the key password, if one was used.
3. Bump versions, commit, and push a new version tag. The release workflow generates a public-key configuration overlay, signs updater artifacts and uploads `latest.json`. Build jobs run serially to prevent concurrent updates to that shared manifest. Without these keys the release still builds installers, just without signed updates; setting only one of them fails the release.
4. Review all platform assets, signatures and `latest.json`, then publish the draft. Test an upgrade from an older signed build before announcing automatic updates.

The updater endpoint is the public GitHub release's `latest.json`. Existing versions without an updater need one manual upgrade first. Linux in-app installation requires running the AppImage; package-manager installations should be updated through their package manager. Tauri update signatures are separate from Apple notarization and Windows code signing.

## License

Git Context is licensed under the [MIT License](LICENSE).

Copyright (c) 2026 Git Context. Third-party dependencies retain their own licenses and notices.
