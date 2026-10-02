## Git Context v0.1.3

Keep your profile switcher available in the macOS menu bar after quitting from the Dock.

### What's new

- **Stay available in the menu bar.** Dock Quit, application-menu Quit, and Cmd+Q now hide Git Context's windows while keeping the profile switcher running.
- **A clear full-exit action.** Choose **Quit Git Context Completely** from the menu-bar menu to stop the app entirely. Force Quit still stops the process.
- **Reopen when needed.** Click the Dock icon or choose **Open Git Context** from the menu bar to restore the main window.

Windows and Linux exit behaviour is unchanged. Your active Git/SSH configuration remains applied when the app exits.

### How to update

**Using the signed v0.1.2 release?** Open **Quick switch & app updates → Check for updates**, then choose **Install update** when v0.1.3 is available.

After installation, choose **Quit Git Context Completely** from the menu-bar menu (called **Quit** in v0.1.2), then reopen the app. Dock Quit and Cmd+Q keep the new version running in the background, so use the menu-bar full-exit action when a restart is required.

**Using v0.1.1 or an unsigned build?** Download and install v0.1.3 manually once to enable future signed updates. Linux users should use the AppImage for in-app updates; package installations can use the matching package download.

Updater signatures are separate from macOS notarization and Windows publisher certificates; this release does not add those certificates.

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
