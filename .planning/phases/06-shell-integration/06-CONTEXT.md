# Phase 6: Shell Integration - Context

**Gathered:** 2026-02-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Shell sessions can connect to mac-client for remote access. Users source an init script in their shell config, and their terminal sessions become accessible through the relay. This phase handles the shell-side integration only — mac-client IPC is already complete from Phase 5.

</domain>

<decisions>
## Implementation Decisions

### Session naming
- Use current working directory as session name (e.g., "claude-code-remote")
- No user override — keep it simple, always directory name
- Add PID suffix for disambiguation: "project [12345]"
- Session name updates live as user navigates (cd triggers rename)

### Connection feedback
- On successful connect: print one-line status (e.g., "Connected to Terminal Remote")
- On disconnect (mac-client quit/crash): print one-time warning
- Auto-reconnect when mac-client comes back — no user action needed
- Menu bar shows session count only (e.g., "3 sessions") not full list

### Graceful degradation
- When mac-client not running at startup: print one-line note, continue normally
- Brief timeout (~100ms) when checking for socket — no perceptible delay
- Auto-detect when mac-client starts after shell is already running
- Shell continues working normally regardless of connection state

### Shell support
- Support zsh, bash, and fish from the start
- Init script sourced at end of rc file (after oh-my-zsh, starship, p10k)
- Directory structure: `~/.terminal-remote/` with `init.zsh`, `init.bash`, `init.fish`

### Claude's Discretion
- Reconnection check interval (how often to poll for mac-client)
- Prompt integration approach (passive vs optional indicator)
- Exact message wording for status/warnings
- Implementation of chpwd hook for live directory tracking

</decisions>

<specifics>
## Specific Ideas

- Session format: "directory-name [PID]" for disambiguation
- Success criteria from roadmap: works with starship, p10k, oh-my-zsh — source at end of rc to avoid conflicts
- No errors or delay when mac-client not running — shell must feel native

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-shell-integration*
*Context gathered: 2026-02-06*
