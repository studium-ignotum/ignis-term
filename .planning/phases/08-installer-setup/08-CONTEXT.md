# Phase 8: Installer & Setup - Context

**Gathered:** 2026-02-06
**Status:** Ready for planning

<domain>
## Phase Boundary

End-user onboarding for macOS — a single curl|sh command that installs the mac-client binary, required dependencies (cloudflared, tmux), and configures shell integration. Also provide a Homebrew tap as an alternative install method. Includes uninstall script.

</domain>

<decisions>
## Implementation Decisions

### Install method & hosting
- Two install methods: curl|sh for quick setup, Homebrew tap for native package management
- Binaries hosted on GitHub Releases (pre-built for arm64/x86_64 Mac)
- Script hosted in the GitHub repo (raw.githubusercontent.com)
- Script is idempotent — re-running detects existing install, updates binary, skips already-configured shell

### Dependency handling
- Required deps: cloudflared, tmux
- cloudflared is already integrated in the relay-server code — script just ensures the binary is installed
- Use Homebrew if available for installing deps; if Homebrew missing, fail hard with clear message about what's needed
- Fail hard on any missing dependency that can't be installed — no partial installs

### Shell configuration
- Auto-detect user's current $SHELL and configure only that one
- Supports zsh, bash, fish (matching existing shell-integration scripts)
- No backup of rc files — the source line is minimal and easily removable
- Prompt user during install: "Start at login? (y/n)" for login item registration

### Binary install location
- Claude's Discretion — pick the best location considering the mac-client is an .app bundle with menu bar integration

### Post-install experience
- Full summary of what was installed + next steps
- Show how to start, session code info, how to verify it's working

### Update mechanism
- Homebrew handles updates (brew upgrade)
- curl|sh re-run also works as update (idempotent)

### Uninstall
- Dedicated uninstall script that removes everything cleanly
- Removes binary, shell integration source lines, login item, deps optionally

</decisions>

<specifics>
## Specific Ideas

- User mentioned cloudflared is already integrated inside relay-server code — just needs the binary present
- Existing shell-integration/install.sh can be leveraged or extended for the shell setup portion

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-installer-setup*
*Context gathered: 2026-02-06*
