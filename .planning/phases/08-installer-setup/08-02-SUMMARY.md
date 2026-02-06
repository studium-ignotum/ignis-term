---
phase: 08-installer-setup
plan: 02
subsystem: infra
tags: [homebrew, cask, github-actions, ci-cd, release-pipeline, tar, macos]

# Dependency graph
requires:
  - phase: 05-mac-client
    provides: "mac-client binary and .app bundle structure"
  - phase: 04-relay-server
    provides: "relay-server binary with embedded web UI"
  - phase: 06-shell-integration
    provides: "init.zsh, init.bash, init.fish scripts"
  - phase: 08-01
    provides: "curl|sh install script with download URL convention"
provides:
  - "Homebrew cask for brew install --cask terminal-remote"
  - "GitHub Actions release workflow for automated binary builds"
  - "Consistent archive naming across all install methods"
affects: []

# Tech tracking
tech-stack:
  added: [github-actions, softprops/action-gh-release, pnpm/action-setup]
  patterns: [matrix-build-per-architecture, staging-directory-packaging, cask-with-postflight]

key-files:
  created:
    - homebrew/Casks/terminal-remote.rb
    - .github/workflows/release.yml
  modified: []

key-decisions:
  - "Cask not Formula -- mac-client is .app bundle (GUI), Casks are correct for GUI apps"
  - "pnpm for web UI build -- matches existing pnpm-lock.yaml in relay-server/web-ui"
  - "Staging directory for tar packaging -- more reliable than tar -s path substitution"
  - "Merged two postflight blocks into one -- Homebrew cask only allows one postflight"

patterns-established:
  - "Archive convention: terminal-remote-v{version}-darwin-{arch}.tar.gz"
  - "Matrix build: macos-14 for ARM64, macos-13 for Intel (native compilation)"
  - "Web UI built before relay-server (rust-embed dependency ordering)"

# Metrics
duration: 2min
completed: 2026-02-06
---

# Phase 8 Plan 2: Homebrew Cask & Release Workflow Summary

**Homebrew cask with architecture-specific downloads and GitHub Actions matrix build pipeline for automated arm64/x86_64 releases**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-06T11:07:31Z
- **Completed:** 2026-02-06T11:09:23Z
- **Tasks:** 2
- **Files created:** 2

## Accomplishments
- Homebrew cask definition with on_arm/on_intel download blocks, cloudflared/tmux dependencies, postflight shell integration setup, and launcher script creation
- GitHub Actions release workflow with matrix build for both Mac architectures, web UI build before relay-server, staging directory packaging, and automated GitHub Release creation
- Consistent archive naming convention across install.sh, Homebrew cask, and release workflow

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Homebrew cask definition** - `0a66155` (feat)
2. **Task 2: Create GitHub Actions release workflow** - `7a87dcc` (feat)

## Files Created/Modified
- `homebrew/Casks/terminal-remote.rb` - Homebrew cask for tap-based installation of Terminal Remote .app bundle with relay-server, shell integration, and dependencies
- `.github/workflows/release.yml` - GitHub Actions workflow that builds both binaries on tag push, packages archives, and creates GitHub Release

## Decisions Made
- **Cask not Formula:** The mac-client is a .app bundle (GUI application). Homebrew Casks are the correct mechanism for .app bundles; Formulae are for CLI tools only.
- **pnpm for web UI build:** The relay-server/web-ui directory uses pnpm (pnpm-lock.yaml exists), so the workflow uses pnpm/action-setup and pnpm install --frozen-lockfile.
- **Staging directory for packaging:** Used a staging directory approach instead of tar -s flag for cross-platform reliability when creating release archives.
- **Merged postflight blocks:** Combined the two separate postflight blocks from the plan into a single postflight block, since Homebrew casks should have one postflight.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Merged duplicate postflight blocks into single block**
- **Found during:** Task 1 (Homebrew cask creation)
- **Issue:** Plan specified two separate `postflight do` blocks. While Ruby technically allows this, Homebrew cask convention is a single postflight block, and having two can cause confusion.
- **Fix:** Merged both postflight blocks (shell integration copying and launcher script creation) into a single `postflight do` block.
- **Files modified:** homebrew/Casks/terminal-remote.rb
- **Verification:** File reads correctly with single postflight containing all setup logic.
- **Committed in:** 0a66155

**2. [Rule 2 - Missing Critical] Added pnpm setup step to workflow**
- **Found during:** Task 2 (GitHub Actions workflow creation)
- **Issue:** Plan showed `npm ci && npm run build` for web UI, but the project uses pnpm (pnpm-lock.yaml exists in relay-server/web-ui/).
- **Fix:** Added pnpm/action-setup@v4 step and used `pnpm install --frozen-lockfile` and `pnpm run build` instead of npm commands.
- **Files modified:** .github/workflows/release.yml
- **Verification:** Confirmed pnpm-lock.yaml exists in relay-server/web-ui/, workflow uses correct package manager.
- **Committed in:** 7a87dcc

**3. [Rule 2 - Missing Critical] Used staging directory instead of tar -s**
- **Found during:** Task 2 (GitHub Actions workflow creation)
- **Issue:** Plan initially showed tar with `-s` flag for path substitution, then noted this could be unreliable and recommended staging directory instead.
- **Fix:** Used staging directory approach as recommended in the plan notes.
- **Files modified:** .github/workflows/release.yml
- **Verification:** Archive creation uses mkdir/cp/cd pattern for reliable cross-platform packaging.
- **Committed in:** 7a87dcc

---

**Total deviations:** 3 auto-fixed (1 bug, 2 missing critical)
**Impact on plan:** All fixes necessary for correctness. No scope creep.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 8 (Installer & Setup) is now complete: both curl|sh installer and Homebrew cask + release pipeline are in place
- Phase 7 Plan 3 (07-03) still pending -- web UI full pipeline completion
- Release pipeline is ready to use once a v* tag is pushed
- Homebrew tap repo (studium-ignotum/homebrew-terminal-remote) needs to be created separately with the Casks/ directory

---
*Phase: 08-installer-setup*
*Completed: 2026-02-06*
