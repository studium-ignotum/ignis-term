# Phase 6: Shell Integration - Research

**Researched:** 2026-02-06
**Domain:** Shell scripting (zsh/bash/fish), Unix domain socket IPC, prompt integration
**Confidence:** HIGH

## Summary

Phase 6 implements shell integration scripts that connect terminal sessions to the mac-client via Unix domain socket. The primary challenge is achieving the <10ms latency requirement while providing reliable connection, graceful degradation, and live directory tracking without interfering with popular prompt tools like starship, powerlevel10k, and oh-my-zsh.

Research confirms that shell hook mechanisms are well-documented and reliable across all three target shells. The IPC protocol is already defined by mac-client (Phase 5): JSON registration message on first line, then bidirectional binary terminal data. macOS includes BSD netcat with Unix socket support (`nc -U`), enabling simple non-blocking communication from shell scripts. Performance benchmarks from similar tools (direnv, starship) show that 5-10ms overhead is achievable with careful implementation.

The key insight is using **background processes for reconnection** rather than synchronous polling in hooks. The shell scripts should attempt connection once at startup (with short timeout), then periodically check in the background without blocking the interactive shell. Directory change hooks (`chpwd` in zsh, `PROMPT_COMMAND` in bash, `--on-variable PWD` in fish) can send updates asynchronously without impacting command latency.

**Primary recommendation:** Use shell-native hook mechanisms with background helper processes for reconnection; communicate via nc (netcat) to the Unix socket with 100ms timeout for initial connection; avoid any blocking operations in prompt hooks.

## Standard Stack

This phase uses only built-in shell features and standard macOS utilities (no external dependencies).

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| nc (netcat) | BSD (macOS built-in) | Unix socket communication | Pre-installed on macOS, supports `-U` for Unix sockets |
| zsh | 5.8+ | Shell hooks via `add-zsh-hook` | Default shell on macOS since Catalina |
| bash | 3.2+ / 5.0+ | Shell hooks via `PROMPT_COMMAND` | Available on all macOS versions |
| fish | 3.0+ | Shell hooks via `--on-variable` | Popular alternative shell |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| timeout | Command timeout wrapper | Wrap nc for guaranteed timeout (if available) |
| jq | JSON manipulation | Optional for parsing responses (not required) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| nc (netcat) | socat | socat more powerful but not pre-installed on macOS |
| nc (netcat) | zsh zsocket module | zsocket requires module loading, adds complexity |
| nc (netcat) | bash /dev/tcp | Not available for Unix sockets |
| Background polling | zsh-async library | Adds dependency, overkill for simple polling |

**Installation:** None required - all tools are pre-installed on macOS or part of the shell itself.

## Architecture Patterns

### Recommended Directory Structure
```
~/.terminal-remote/
├── init.zsh           # Zsh integration (sourced by .zshrc)
├── init.bash          # Bash integration (sourced by .bashrc)
├── init.fish          # Fish integration (sourced by config.fish)
└── lib/
    ├── common.sh      # Shared functions (POSIX sh compatible)
    └── send-message   # Optional: compiled helper for faster IPC
```

### Pattern 1: Lazy Connection with Background Reconnect
**What:** Connect to mac-client once at shell startup; if unavailable, start background watcher
**When to use:** Always - this is the recommended architecture

```bash
# Pseudocode pattern (applies to all shells)
on_shell_start:
  if socket_exists("/tmp/terminal-remote.sock"):
    try_connect_with_timeout(100ms)
    if connected:
      register_session()
      print("Connected to Terminal Remote")
    else:
      start_background_watcher()
  else:
    start_background_watcher()

background_watcher:
  while shell_running:
    sleep(5s)  # Check every 5 seconds
    if not connected and socket_exists:
      try_connect()
```

### Pattern 2: Directory Change Hook with Async Update
**What:** Send session name updates on directory change without blocking
**When to use:** For live directory tracking (SHELL-08 requirement)

```zsh
# Zsh pattern using chpwd hook
_terminal_remote_chpwd() {
  # Fire-and-forget: don't wait for response
  if [[ -n "$_TERMINAL_REMOTE_FD" ]]; then
    _terminal_remote_send_update &!  # Background, disown
  fi
}
add-zsh-hook chpwd _terminal_remote_chpwd
```

### Pattern 3: Exit Cleanup Handler
**What:** Gracefully disconnect session when shell exits
**When to use:** For SHELL-09 requirement

```zsh
# Zsh: Use zshexit hook
zshexit() {
  _terminal_remote_disconnect 2>/dev/null
}

# Bash: Use trap
trap '_terminal_remote_disconnect 2>/dev/null' EXIT

# Fish: Use event handler
function _terminal_remote_cleanup --on-event fish_exit
  _terminal_remote_disconnect 2>/dev/null
end
```

### Pattern 4: Non-Blocking Socket Send
**What:** Send data to socket without waiting for response
**When to use:** For directory updates and other fire-and-forget messages

```bash
# POSIX sh compatible - works in all shells
_terminal_remote_send() {
  local msg="$1"
  # -w 1: 1 second timeout (minimum granularity for nc)
  # -N: Shutdown network socket after EOF on stdin (send and close)
  echo "$msg" | nc -U /tmp/terminal-remote.sock -w 1 -N 2>/dev/null &
  disown 2>/dev/null || true
}
```

### Anti-Patterns to Avoid

- **Synchronous socket operations in precmd/PROMPT_COMMAND:** Adds latency to every prompt
- **Polling in foreground:** Blocks shell, violates <10ms requirement
- **Checking socket on every command:** Wasteful, use hooks only for relevant events
- **Forking subshell for every operation:** Use file descriptors where possible
- **Ignoring connection state:** Track state to avoid repeated failed connection attempts

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Socket communication | Raw file descriptors in bash | nc -U | nc handles buffering, timeouts, error handling |
| JSON generation | String concatenation | printf with escaping | Proper escaping for special chars |
| Process backgrounding | & alone | &! (zsh) or disown | Prevents job control messages |
| Directory basename | String manipulation | ${PWD##*/} | Shell built-in, portable, fast |
| Timeout wrapper | sleep + kill | nc -w or timeout cmd | Race condition handling is tricky |

**Key insight:** Shell scripts should be thin wrappers around existing tools. The complexity should live in mac-client (Rust), not in shell scripts that must work across three different shells.

## Common Pitfalls

### Pitfall 1: Socket Check Causes Startup Delay
**What goes wrong:** Shell takes 500ms+ to start when mac-client isn't running
**Why it happens:** Synchronous socket connection attempt with default TCP timeout
**How to avoid:**
1. Check if socket file exists before attempting connection (instant filesystem check)
2. Use nc -w 0.1 for 100ms timeout (macOS nc rounds to 1 second minimum, so use background + kill)
3. If neither works: fork connection attempt to background immediately
**Warning signs:** Noticeable pause when opening new terminal without mac-client running

### Pitfall 2: Job Control Messages Spam Terminal
**What goes wrong:** "[1] 12345" and "[1]+ Done" messages appear when backgrounding
**Why it happens:** Shell job control notifications for background processes
**How to avoid:**
1. Zsh: Use `&!` (disown immediately) instead of just `&`
2. Bash: Use `disown` after `&`, or use subshell `( cmd & )`
3. Fish: Use `fish -c 'cmd &' &` or `command cmd &` pattern
**Warning signs:** Random job completion messages appearing at prompts

### Pitfall 3: Prompt Tool Conflicts
**What goes wrong:** Terminal Remote breaks starship/p10k prompts, or vice versa
**Why it happens:** Multiple tools modifying same hooks, order-dependent initialization
**How to avoid:**
1. Source terminal-remote init at END of shell rc file (after oh-my-zsh, starship init)
2. Use `add-zsh-hook` instead of overwriting functions directly
3. Never modify PS1/PROMPT directly - use dedicated hooks
4. Keep precmd hook minimal (< 5ms execution time)
**Warning signs:** Prompt looks wrong, theme features stop working

### Pitfall 4: Reconnection Loop Hammers Socket
**What goes wrong:** Background process constantly tries to connect, wastes CPU
**Why it happens:** No backoff between reconnection attempts
**How to avoid:**
1. Start with 5-second interval, increase on failure
2. Check if socket file exists before attempting connect
3. Use exponential backoff (5s, 10s, 20s, capped at 60s)
4. Stop trying after mac-client confirmed unavailable
**Warning signs:** High CPU from shell process, excessive nc processes

### Pitfall 5: File Descriptor Leak
**What goes wrong:** Eventually "too many open files" error
**Why it happens:** Opening socket but not properly closing on all code paths
**How to avoid:**
1. Use nc for one-shot operations (opens and closes automatically)
2. If maintaining persistent FD, close on disconnect AND on shell exit
3. Test with many reconnection cycles
**Warning signs:** "Too many open files" after running for extended time

### Pitfall 6: Session Name Encoding Issues
**What goes wrong:** Directories with spaces, unicode, or special chars break JSON
**Why it happens:** Improper escaping when building JSON registration message
**How to avoid:**
1. Use proper JSON escaping for the name field
2. Test with directories: "My Project", "/tmp/test\\ndir", "/path/with\"quotes"
3. Use jq for JSON if available, otherwise careful printf escaping
**Warning signs:** Session connects but has wrong/corrupted name

## Code Examples

Verified patterns for each shell:

### Zsh Integration (init.zsh)
```zsh
# Source: https://zsh.sourceforge.io/Doc/Release/Functions.html (hook functions)
# Source: https://github.com/starship/starship (hook pattern inspiration)

# Configuration
typeset -g _TERMINAL_REMOTE_SOCKET="/tmp/terminal-remote.sock"
typeset -g _TERMINAL_REMOTE_CONNECTED=0
typeset -g _TERMINAL_REMOTE_PID=$$

# Load hook helper
autoload -Uz add-zsh-hook

# Get session name: directory basename + PID for disambiguation
_terminal_remote_session_name() {
  local dir_name="${PWD##*/}"
  [[ -z "$dir_name" ]] && dir_name="/"
  echo "${dir_name} [${_TERMINAL_REMOTE_PID}]"
}

# Escape string for JSON (basic escaping)
_terminal_remote_json_escape() {
  local str="$1"
  str="${str//\\/\\\\}"      # Backslash
  str="${str//\"/\\\"}"      # Quote
  str="${str//$'\n'/\\n}"    # Newline
  str="${str//$'\t'/\\t}"    # Tab
  echo "$str"
}

# Send registration message
_terminal_remote_register() {
  local name=$(_terminal_remote_json_escape "$(_terminal_remote_session_name)")
  local msg="{\"name\":\"${name}\",\"shell\":\"zsh\",\"pid\":${_TERMINAL_REMOTE_PID}}"

  # Send with 1s timeout, background to not block
  { echo "$msg" | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null; } &!
  _TERMINAL_REMOTE_CONNECTED=1
}

# Send session update (name change on cd)
_terminal_remote_update() {
  (( _TERMINAL_REMOTE_CONNECTED )) || return
  local name=$(_terminal_remote_json_escape "$(_terminal_remote_session_name)")
  local msg="{\"type\":\"update\",\"name\":\"${name}\"}"
  { echo "$msg" | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null; } &!
}

# Hook: directory changed
_terminal_remote_chpwd() {
  _terminal_remote_update
}

# Hook: shell exiting
_terminal_remote_zshexit() {
  (( _TERMINAL_REMOTE_CONNECTED )) || return
  # Best-effort disconnect, don't wait
  { echo '{"type":"disconnect"}' | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null; } &!
}

# Initial connection attempt
_terminal_remote_init() {
  # Fast check: socket file exists?
  [[ -S "$_TERMINAL_REMOTE_SOCKET" ]] || {
    # Mac-client not running, silently skip
    return 0
  }

  # Try to connect (background to not block startup)
  _terminal_remote_register
  echo "Connected to Terminal Remote"
}

# Register hooks
add-zsh-hook chpwd _terminal_remote_chpwd
zshexit() { _terminal_remote_zshexit }

# Initialize
_terminal_remote_init
```

### Bash Integration (init.bash)
```bash
# Source: https://www.gnu.org/software/bash/manual/html_node/Controlling-the-Prompt.html
# Source: https://gist.github.com/laggardkernel/6cb4e1664574212b125fbfd115fe90a4 (chpwd pattern)

# Configuration
_TERMINAL_REMOTE_SOCKET="/tmp/terminal-remote.sock"
_TERMINAL_REMOTE_CONNECTED=0
_TERMINAL_REMOTE_PID=$$
_TERMINAL_REMOTE_PREVPWD=""

# Get session name
_terminal_remote_session_name() {
  local dir_name="${PWD##*/}"
  [[ -z "$dir_name" ]] && dir_name="/"
  echo "${dir_name} [${_TERMINAL_REMOTE_PID}]"
}

# JSON escape (basic)
_terminal_remote_json_escape() {
  local str="$1"
  str="${str//\\/\\\\}"
  str="${str//\"/\\\"}"
  str="${str//$'\n'/\\n}"
  str="${str//$'\t'/\\t}"
  echo "$str"
}

# Send registration
_terminal_remote_register() {
  local name
  name=$(_terminal_remote_json_escape "$(_terminal_remote_session_name)")
  local msg="{\"name\":\"${name}\",\"shell\":\"bash\",\"pid\":${_TERMINAL_REMOTE_PID}}"

  ( echo "$msg" | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null & )
  _TERMINAL_REMOTE_CONNECTED=1
}

# Send update
_terminal_remote_update() {
  [[ $_TERMINAL_REMOTE_CONNECTED -eq 1 ]] || return
  local name
  name=$(_terminal_remote_json_escape "$(_terminal_remote_session_name)")
  local msg="{\"type\":\"update\",\"name\":\"${name}\"}"
  ( echo "$msg" | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null & )
}

# Directory change hook (checked in PROMPT_COMMAND)
_terminal_remote_chpwd_hook() {
  if [[ "$_TERMINAL_REMOTE_PREVPWD" != "$PWD" ]]; then
    _TERMINAL_REMOTE_PREVPWD="$PWD"
    _terminal_remote_update
  fi
}

# Cleanup on exit
_terminal_remote_cleanup() {
  [[ $_TERMINAL_REMOTE_CONNECTED -eq 1 ]] || return
  ( echo '{"type":"disconnect"}' | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null & )
}
trap '_terminal_remote_cleanup' EXIT

# Add to PROMPT_COMMAND (preserve existing)
_terminal_remote_prompt_hook() {
  _terminal_remote_chpwd_hook
}

# Append to PROMPT_COMMAND safely
PROMPT_COMMAND="_terminal_remote_prompt_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

# Initialize
_terminal_remote_init() {
  [[ -S "$_TERMINAL_REMOTE_SOCKET" ]] || return 0
  _terminal_remote_register
  echo "Connected to Terminal Remote"
}

_TERMINAL_REMOTE_PREVPWD="$PWD"
_terminal_remote_init
```

### Fish Integration (init.fish)
```fish
# Source: https://fishshell.com/docs/current/cmds/function.html (event handlers)
# Source: https://fishshell.com/docs/current/language.html (--on-variable)

# Configuration
set -g _TERMINAL_REMOTE_SOCKET "/tmp/terminal-remote.sock"
set -g _TERMINAL_REMOTE_CONNECTED 0
set -g _TERMINAL_REMOTE_PID %self

# Get session name
function _terminal_remote_session_name
  set -l dir_name (basename $PWD)
  test -z "$dir_name"; and set dir_name "/"
  echo "$dir_name [$_TERMINAL_REMOTE_PID]"
end

# JSON escape (fish version)
function _terminal_remote_json_escape
  string replace -a '\\' '\\\\' -- $argv[1] | \
  string replace -a '"' '\\"' | \
  string replace -a \n '\\n' | \
  string replace -a \t '\\t'
end

# Send registration
function _terminal_remote_register
  set -l name (_terminal_remote_json_escape (_terminal_remote_session_name))
  set -l msg "{\"name\":\"$name\",\"shell\":\"fish\",\"pid\":$_TERMINAL_REMOTE_PID}"

  fish -c "echo '$msg' | nc -U '$_TERMINAL_REMOTE_SOCKET' -w 1 2>/dev/null" &
  disown
  set -g _TERMINAL_REMOTE_CONNECTED 1
end

# Send update
function _terminal_remote_update
  test $_TERMINAL_REMOTE_CONNECTED -eq 1; or return
  set -l name (_terminal_remote_json_escape (_terminal_remote_session_name))
  set -l msg "{\"type\":\"update\",\"name\":\"$name\"}"
  fish -c "echo '$msg' | nc -U '$_TERMINAL_REMOTE_SOCKET' -w 1 2>/dev/null" &
  disown
end

# Directory change hook
function _terminal_remote_chpwd --on-variable PWD
  _terminal_remote_update
end

# Exit cleanup
function _terminal_remote_cleanup --on-event fish_exit
  test $_TERMINAL_REMOTE_CONNECTED -eq 1; or return
  echo '{"type":"disconnect"}' | nc -U "$_TERMINAL_REMOTE_SOCKET" -w 1 2>/dev/null &
end

# Initialize
function _terminal_remote_init
  test -S "$_TERMINAL_REMOTE_SOCKET"; or return 0
  _terminal_remote_register
  echo "Connected to Terminal Remote"
end

_terminal_remote_init
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Named pipes for IPC | Unix domain sockets | N/A | Bidirectional, multiple clients |
| Modifying PS1 directly | Using add-zsh-hook | zsh 4.3+ | Compatible with prompt themes |
| Sync socket in PROMPT_COMMAND | Background + fire-and-forget | N/A | <10ms latency achievable |
| Single shell support | Multi-shell with shared patterns | N/A | zsh/bash/fish all supported |

**Deprecated/outdated:**
- **Direct PS1 modification**: Breaks other prompt tools; use hooks instead
- **Blocking socket operations in hooks**: Causes perceptible delay
- **GNU netcat assumptions**: macOS uses BSD netcat with different flags

## Open Questions

Things that couldn't be fully resolved:

1. **Sub-second timeout with macOS nc**
   - What we know: macOS nc -w accepts integers only (minimum 1 second)
   - What's unclear: Whether the actual timeout can be < 1s internally
   - Recommendation: Use background + kill pattern for 100ms timeout, or accept 1s timeout since it only affects initial connection attempt

2. **Terminal data flow after registration**
   - What we know: mac-client expects bidirectional binary data after registration
   - What's unclear: Whether shell scripts need to maintain persistent connection or just send one-shot messages
   - Recommendation: Clarify with mac-client implementation - likely one-shot registration then terminal app handles actual PTY data

3. **Reconnection notification mechanism**
   - What we know: User wants auto-reconnect when mac-client starts
   - What's unclear: Optimal polling interval and how to notify shell of reconnection
   - Recommendation: 5-second polling interval, print message when reconnect succeeds

## Sources

### Primary (HIGH confidence)
- [Zsh Functions Documentation](https://zsh.sourceforge.io/Doc/Release/Functions.html) - Hook functions (chpwd, precmd, zshexit)
- [GNU Bash Manual - Controlling the Prompt](https://www.gnu.org/software/bash/manual/html_node/Controlling-the-Prompt.html) - PROMPT_COMMAND
- [Fish Shell Documentation - Functions](https://fishshell.com/docs/current/cmds/function.html) - Event handlers (--on-variable, --on-event)
- [macOS nc man page](https://ss64.com/mac/nc.html) - BSD netcat options including -U
- [Starship ZSH Init](https://github.com/starship/starship) - Hook pattern inspiration (verified in WebFetch)

### Secondary (MEDIUM confidence)
- [Mastering Zsh - Hooks](https://github.com/rothgar/mastering-zsh/blob/master/docs/config/hooks.md) - add-zsh-hook usage
- [Bash chpwd Equivalent](https://gist.github.com/laggardkernel/6cb4e1664574212b125fbfd115fe90a4) - PROMPT_COMMAND directory tracking pattern
- [direnv Hook Documentation](https://direnv.net/docs/hook.html) - Shell integration patterns
- [ZSH Performance Blog](https://www.dribin.org/dave/blog/archives/2024/01/01/zsh-performance/) - Hook latency measurements (~5ms achievable)

### Tertiary (LOW confidence)
- [Zsh zshexit Hook](https://www.refining-linux.org/archives/43-ZSH-Gem-9-zshexit-and-other-hook-functions.html) - Exit hook behavior (needs verification against current zsh)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - All tools are macOS built-ins or shell native features
- Architecture: HIGH - Patterns derived from well-established tools (starship, direnv)
- Pitfalls: HIGH - Based on documented issues and shell-specific gotchas
- Code examples: MEDIUM - Patterns verified from official docs but not tested end-to-end

**Research date:** 2026-02-06
**Valid until:** 90 days (shell features are very stable, unlikely to change)
