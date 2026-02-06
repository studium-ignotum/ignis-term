---
phase: 08-installer-setup
plan: 01
subsystem: infra
tags: [bash, installer, curl-pipe-sh, launchagent, homebrew, shell-integration]

# Dependency graph
requires:
  - phase: 06-shell-integration
    provides: Shell init scripts (init.zsh, init.bash, init.fish) and install pattern
provides:
  - curl|sh installer for complete Terminal Remote stack
  - Clean uninstall script reversing all install actions
  - LaunchAgent for login startup
  - Launcher script for manual start
affects: [08-02-homebrew-cask, release-workflow, README]

# Tech tracking
tech-stack:
  added: [launchctl, LaunchAgent plist]
  patterns: [curl-pipe-sh installer, idempotent shell rc modification, trap-based cleanup]

key-files:
  created:
    - scripts/install.sh
    - scripts/uninstall.sh
  modified: []

key-decisions:
  - "Hard fail on missing Homebrew -- no partial installs"
  - "Shell integration copied from release archive for version matching"
  - "grep -qF for idempotent source line insertion"
  - "LaunchAgent for login startup instead of login items API"
  - "Default no for login prompt when piped (curl|sh)"

patterns-established:
  - "Installer constants: INSTALL_DIR=$HOME/.terminal-remote, APP_DIR=$HOME/Applications"
  - "Color codes consistent with setup.sh/start.sh"
  - "Confirmation prompt pattern for destructive operations"

# Metrics
duration: 2min
completed: 2026-02-06
---

# Phase 8 Plan 01: Installer & Uninstaller Summary

**Production curl|sh installer with arch detection, Homebrew dep management, GitHub Releases download, idempotent shell config, and clean uninstall**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-06T11:06:26Z
- **Completed:** 2026-02-06T11:08:33Z
- **Tasks:** 2
- **Files created:** 2

## Accomplishments
- Complete curl|sh installer handling the full Terminal Remote stack (binaries, shell integration, deps, login item)
- Idempotent re-run support: updates binaries without duplicating shell rc lines
- Clean uninstall script that reverses every install action with graceful handling of missing components
- Architecture detection (arm64/x86_64), Homebrew hard requirement, GitHub Releases version resolution

## Task Commits

Each task was committed atomically:

1. **Task 1: Create the main install script** - `eb34651` (feat)
2. **Task 2: Create the uninstall script** - `a8eb672` (feat)

## Files Created/Modified
- `scripts/install.sh` - Main curl|sh installer (287 lines): arch detection, Homebrew check, dep install, binary download from GitHub Releases, shell config, LaunchAgent, launcher
- `scripts/uninstall.sh` - Clean uninstall script (154 lines): process kill, LaunchAgent removal, app removal, shell line cleanup, optional dep removal

## Decisions Made
- **Hard fail on missing Homebrew:** No partial installs -- cloudflared and tmux are required dependencies that need Homebrew
- **Shell integration from release archive:** Init scripts are copied from the downloaded release tarball (not fetched separately) to ensure version matching with binaries
- **grep -qF for duplicate prevention:** Searches for `terminal-remote/init.` pattern to detect existing source lines regardless of quoting style
- **LaunchAgent over SMAppService:** Plist-based LaunchAgent is simpler, works across macOS versions, and is easily reversible
- **Default "n" for piped stdin:** When running via curl|sh, login prompt defaults to "n" since interactive read is unavailable
- **sed -i '' for macOS compatibility:** Uses macOS-native sed syntax for in-place editing of rc files during uninstall

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Install/uninstall scripts ready; require GitHub Releases with correct tarball structure to function
- Release workflow (CI/CD) needed to produce `terminal-remote-$VERSION-darwin-$ARCH.tar.gz` archives
- Homebrew cask (08-02) can reference install.sh or provide alternative installation method

---
*Phase: 08-installer-setup*
*Completed: 2026-02-06*
