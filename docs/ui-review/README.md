# One-click activation UI review — 2026-09-30

Activation now uses a single `activate_profile` native command. It creates a fresh plan and applies it while holding the shared state lock. Existing validation, configuration locks, durable backup, post-write checks and rollback remain intact. The user click is the authorization; no second confirmation is displayed.

Configuration Details is a read-only right-side sheet, available on each profile row and the persistent active-context card. It shows the existing before/after data without applying it. The profile switcher lives in the sidebar. The active-context card is visible on Profiles, SSH keys and Settings.

Desktop resizing retains a 220px sidebar instead of switching to a 56px icon rail at 900px. Main content is bounded to 1160px, with stable scrollbar space and no dimension transitions. The side sheet uses a short transform/opacity entrance and honors reduced motion.

Validation:
- 10 frontend tests passed, including one-click activation, read-only details, activation failure, and preventing duplicate clicks while keeping the previous active identity until success.
- 38 Rust tests passed; Clippy passed with warnings denied.
- Browser fixture checks passed at 820×600, 1240×850 and 1920×1080: fixed sidebar width, no horizontal overflow, row/sidebar one-click activation, read-only sheet and reduced-motion behavior.
- Screenshots use demo data and do not change the user's Git/SSH configuration.
- Native macOS app bundle rebuilt. OS-level maximize animation has not been independently instrumented; the tested improvement is stable app layout across viewport sizes.

The existing launch video predates this UX change and still shows the old confirmation step. Re-record those scenes before using that video to represent this version.
