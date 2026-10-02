## Git Context v0.1.3

Fixes updater permissions and misleading installation errors.

### What's fixed

- The main window now has the resource-close permission needed to release updater resources.
- A cleanup failure no longer turns a successful installation into an “Update failed” message.
- Repeated checks and window teardown safely handle cleanup failures.
- After installation, the restart message stays visible. Use **Quit Git Context Completely** (or **Quit** in older versions) from the tray/menu bar, then reopen the app.

### macOS menu-bar behaviour

Dock Quit, application-menu Quit, and Cmd+Q hide the windows while keeping the profile switcher running. Use **Quit Git Context Completely** from the menu bar to fully exit. Click the Dock icon or choose **Open Git Context** to reopen the window.

### Updating

If you already installed the earlier v0.1.3 build, manually reinstall the corrected v0.1.3 installer. The updater does not offer a same-version replacement.

If an older version reports `plugin:resources|close not allowed by ACL` after installing, fully quit from the tray/menu bar and reopen first: the installation may already have completed. If it has not, manually install v0.1.3 from Assets. Existing installations cannot receive permission changes until the new build is installed.

Unsigned builds require a manual installation. Linux package users can update with the matching package; AppImage users can use the in-app updater. Updater signing does not add macOS notarization or Windows publisher certificates.

### Downloads

Expand **Assets** and choose the installer matching your operating system and processor:

| Platform | Download |
| --- | --- |
| macOS, Apple Silicon | `.dmg` with `aarch64` |
| macOS, Intel | `.dmg` with `x64` |
| Windows x64 | `x64-setup.exe` or x64 `.msi` |
| Windows ARM64 | `arm64-setup.exe` |
| Ubuntu / Debian | `.deb` with `amd64` or `arm64` |
| Other Linux distributions | `.AppImage` or a compatible `.rpm`, matching your architecture |

The source-code archives are not installers. Updater archives, `.sig` files, and `latest.json` are used by the app's updater.

**Requirements:** Git and OpenSSH installed and available on your `PATH`. Backups and configuration history remain available in Settings. Your SSH private keys stay on your machine.

[User guide](https://github.com/PandyaPreet/GitContext#readme) · [Report an issue](https://github.com/PandyaPreet/GitContext/issues)
