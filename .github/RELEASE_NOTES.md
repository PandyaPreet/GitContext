## Git Context: your Git identities, finally under control

Git Context is a small desktop app for developers who use more than one GitHub or GitLab account: work and personal, several clients, open source. It sets the right name, email and SSH key for each repository, so you stop pushing commits as the wrong person.

### What's new in 0.1.1

- **Generate SSH keys in-app.** Checks your GitHub or GitLab username, creates an Ed25519 key, and opens your provider's SSH settings so you can paste it.
- **Identity health check.** Spots missing keys, author/SSH conflicts and configuration changed outside the app, and offers **Reapply** for drift.
- **Verify active identity.** Confirms the global author, managed SSH configuration and key authentication in one step.
- **Quick switcher.** Press `Cmd+Shift+G` / `Ctrl+Shift+G` to switch profiles from a compact picker.
- **Profile colours everywhere.** Sidebar, avatar and tray icon follow the active profile; macOS also shows its name in the menu bar.
- **Fixes.** SSH configuration checks now work on Windows.

### Highlights

- **Profiles for every identity.** GitHub and GitLab, including self-hosted hosts and custom SSH ports.
- **Per-repository assignment.** Sets `user.name`, `user.email` and an SSH host alias, and can rewrite the remote if you want it to.
- **Preview before apply.** Every change to `~/.ssh/config`, `~/.gitconfig` or a repository's config is shown as a before/after diff first.
- **Undo.** Each applied change is journaled and can be rolled back from Settings → Configuration history.
- **SSH verification.** Confirms that a key really authenticates as the account you expect.
- **Global default profile.** Switch your machine-wide identity from the tray in one click.
- **Command palette.** Press `⌘K` / `Ctrl K` to find profiles and actions.
- **Launch in context.** Open a repository in VS Code, Terminal, iTerm, Windows Terminal, PowerShell or CMD once its identity has been checked.
- **Local only.** No account, no telemetry, no network listener. Your keys never leave your machine.

### Downloads

| Platform | File |
| --- | --- |
| macOS, Apple Silicon | `Git.Context_*_aarch64.dmg` |
| macOS, Intel | `Git.Context_*_x64.dmg` |
| Windows x64 | `Git.Context_*_x64-setup.exe` (or `.msi`) |
| Windows ARM64 | `Git.Context_*_arm64-setup.exe` |
| Linux x64 | `.AppImage`, `.deb` (`amd64`), `.rpm` (`x86_64`) |
| Linux ARM64 | `.AppImage`, `.deb` (`arm64`), `.rpm` (`aarch64`) |

### First launch

These builds are not yet notarized or code signed by a paid certificate, so your OS may warn you the first time:

- **macOS:** right-click the app → **Open** → **Open**. If macOS says the app is damaged, run `xattr -dr com.apple.quarantine "/Applications/Git Context.app"`.
- **Windows:** if SmartScreen appears, click **More info** → **Run anyway**.
- **Linux (AppImage):** make the downloaded file executable with `chmod +x`, then run it.

**Requirements:** Git and OpenSSH installed and on your `PATH`.

See the [README](https://github.com/PandyaPreet/GitContext#readme) for the full guide. Found a bug? [Open an issue](https://github.com/PandyaPreet/GitContext/issues).
