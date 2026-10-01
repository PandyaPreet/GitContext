## Git Context v0.1.2

More reliable GitLab SSH verification and the first release with signed in-app updates. This release includes all changes planned for v0.1.2, which was not published.

Git Context helps you switch your global Git author and SSH identity between work, personal, and client profiles. Repository-local overrides and HTTPS credentials remain independent.

### What's new

- **Smoother first-time SSH verification.** Explicit verification now records a previously unknown host key in `known_hosts`, matching GitShift. Changed host keys remain blocked. This trusts the first connection; it does not independently validate a new fingerprint.
- **More reliable GitLab greeting detection.** Handles capitalization, surrounding whitespace, and connection banners while still checking the authenticated account against your profile username.
- **Fresh, profile-specific authentication.** Verification ignores user/system SSH rules and connection reuse, uses the selected key, and respects custom GitLab hosts and ports.
- **Clearer troubleshooting.** Missing keys, DNS failures, refused connections, changed host keys, and SSH-agent signing failures have more useful messages. Verification allows 15 seconds to connect and up to 30 seconds overall.
- **Signed updates.** Release builds use the configured Tauri signing key to produce verifiable updater downloads and the update manifest.

### Updating from v0.1.1 or earlier

**Download and install v0.1.3 manually once.** Older builds were distributed without an updater public key and cannot install this first signed update through the app.

After installing this updater-enabled release, open **Quick switch & app updates → Check for updates** for future releases. When an update is available, review its notes, click **Install update**, and follow the restart instructions. Linux users should use the AppImage for the in-app updater; package installations can be updated with the matching package download.

Updater signatures verify update authenticity. They are separate from macOS notarization and Windows publisher certificates; this release does not add those certificates.

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
