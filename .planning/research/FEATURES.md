# Features Research: Universal Terminal Remote v2.0

**Domain:** Remote terminal access with universal terminal support
**Researched:** 2026-02-06
**Focus:** NEW v2.0 features (menu bar app, shell integration, universal terminal support)
**Confidence:** MEDIUM-HIGH (based on comparable products and Apple HIG)

## Existing Features (Validated in v1.0)

Features already proven in the Node.js/SvelteKit prototype. These carry forward:

| Feature | Status | Notes |
|---------|--------|-------|
| WebSocket relay architecture | Validated | Works well |
| Session code authentication | Validated | Stateless, secure enough for single-user |
| xterm.js terminal emulation | Validated | Colors, cursor, escape sequences work |
| Real-time bidirectional I/O | Validated | Latency acceptable |
| iTerm2 tab switching | Validated | Works but limited to iTerm2 only |
| Connection status indicator | Validated | Essential UX element |
| Auto-reconnect on network interruption | Validated | Grace period approach works |

---

## Menu Bar App Features

### Table Stakes

Features users expect from a macOS menu bar app. Missing = feels broken.

| Feature | Why Expected | Complexity | Testable Criteria |
|---------|--------------|------------|-------------------|
| **Status icon in menu bar** | Core visibility mechanism; user must know app is running | Low | Icon visible in menu bar when app running |
| **Click to open menu** | Standard macOS interaction pattern | Low | Single click opens dropdown menu |
| **Session code visible in menu** | Primary information user needs; why they opened menu | Low | 6-digit code displayed prominently |
| **Copy session code action** | Users need to share code with remote browser | Low | "Copy Code" button copies to clipboard with confirmation |
| **Connection status indicator** | User must know if connected to relay | Low | Visual indicator (icon color or text) shows connected/disconnected |
| **Quit option** | Standard app control | Low | "Quit" menu item terminates app |
| **Start at login option** | Background utilities should persist across restarts | Low | Preference toggle, writes to Login Items |
| **Template image for icon** | Automatic dark/light mode adaptation (Apple HIG) | Low | Icon readable in both light and dark menu bars |
| **No Dock icon** | Menu bar apps shouldn't clutter Dock | Low | App does not appear in Dock (LSUIElement) |
| **Session count** | User should know how many terminals are connected | Low | "3 sessions" or similar displayed |

### Differentiators

Features that make the menu bar app excellent, not just functional.

| Feature | Value Proposition | Complexity | Testable Criteria |
|---------|-------------------|------------|-------------------|
| **Session list in menu** | See which terminals are connected without opening browser | Medium | Menu shows list of connected shell sessions with names |
| **Click session to open in browser** | Quick access to specific terminal | Medium | Clicking session opens browser to that terminal |
| **Notification when new session connects** | Awareness of terminal activity | Low | macOS notification appears when shell connects |
| **QR code for session code** | Easy sharing to mobile devices | Low | "Show QR Code" generates scannable code |
| **Keyboard shortcut to copy code** | Power user efficiency | Low | Global hotkey (configurable) copies code |
| **Auto-regenerate session code** | Security: code rotation every N minutes | Medium | Code changes automatically, connected sessions unaffected |
| **Manual code regeneration** | Security: user-initiated rotation | Low | "New Code" button generates fresh code |

### Menu Bar Anti-Features

Features to explicitly NOT build in the menu bar app.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **NSPopover for menu** | Feels non-native: delays, wrong dismiss behavior | Use NSMenu with custom views |
| **Full app window** | Menu bar apps should be lightweight; complexity belongs in browser UI | Keep all complexity in web UI |
| **Terminal rendering in menu** | Performance drain, tiny viewport useless | Show session names only; render terminals in browser |
| **Multiple relay support** | Complexity for marginal value | Single relay URL, configurable in preferences |
| **Dark mode toggle** | System controls this automatically | Use template images, respect system setting |
| **Update notifications in menu** | Annoying, breaks flow | Silent auto-update or separate preference |

---

## Shell Integration Features

### Table Stakes

Features users expect from shell integration that "just works."

| Feature | Why Expected | Complexity | Testable Criteria |
|---------|--------------|------------|-------------------|
| **Auto-connect when shell starts** | Core value prop: no manual steps required | Low | New terminal window auto-connects to mac-client |
| **Works with zsh** | Default macOS shell since Catalina | Low | `source ~/.terminal-remote/init.zsh` works |
| **Works with bash** | Second most common shell | Low | `source ~/.terminal-remote/init.bash` works |
| **Silent when mac-client not running** | Don't break shell when app not running | Low | No errors or delays if mac-client offline |
| **No prompt interference** | Should not break starship/p10k/custom prompts | Medium | Custom prompts render correctly |
| **No command delay** | Integration must be imperceptible | Low | Commands execute with <10ms added latency |
| **Works in any terminal app** | Core v2 value: universal support | Medium | Same script works in iTerm2, Terminal.app, VS Code, Warp, etc. |
| **Session naming** | User must identify which terminal is which | Low | Session named from $PWD or $TERM_PROGRAM |
| **Graceful disconnect** | Shell exit should clean up session | Low | Closing terminal removes session from mac-client |

### Differentiators

Features that make shell integration excellent.

| Feature | Value Proposition | Complexity | Testable Criteria |
|---------|-------------------|------------|-------------------|
| **Automatic installation** | One-liner setup | Low | `curl ... | sh` adds line to .zshrc |
| **Detection of tmux/screen** | Avoid double-wrapping terminal multiplexers | Medium | Inside tmux, integration handles correctly |
| **Working directory tracking** | Remote user sees current directory | Low | Directory changes reflected in web UI |
| **Command start/end events** | Enable command timing, success/fail indicators | Medium | Web UI shows when commands are running |
| **Process name tracking** | Show what's running (vim, npm, etc.) | Medium | Web UI shows active process name |
| **Conditional connect** | Only connect if env var set | Low | `TERMINAL_REMOTE=1 zsh` for manual control |
| **Fish shell support** | Popular among developers | Low | `source ~/.terminal-remote/init.fish` works |

### Shell Integration Anti-Features

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Modifying PS1/PROMPT** | Breaks custom prompts, causes conflicts | Use PRECMD/PREEXEC hooks only |
| **Background process in shell** | Complicates shell lifecycle, job control | Connect to mac-client Unix socket |
| **Per-terminal detection** | Complexity, maintenance burden | Shell-level integration works everywhere |
| **Auto-update of integration script** | Security concern, unexpected changes | Manual update, version check only |
| **Clipboard sync via shell** | Complex, security issues | Use browser clipboard directly |

### Technical Implementation Notes

Based on research of kitty, iTerm2, and VS Code shell integration:

**Hook Mechanism (zsh):**
```zsh
# Use standard zsh hooks, don't modify PROMPT
autoload -Uz add-zsh-hook
add-zsh-hook precmd  _terminal_remote_precmd   # Before prompt
add-zsh-hook preexec _terminal_remote_preexec  # Before command
```

**Connection Mechanism:**
- Connect to mac-client via Unix socket (`~/.terminal-remote/socket`)
- If socket doesn't exist, silently no-op
- Send PTY file descriptor for I/O routing
- Send metadata (PWD, TERM_PROGRAM, etc.)

**Session Lifecycle:**
1. Shell starts, sources integration script
2. Script checks for mac-client socket
3. If found, registers session with metadata
4. Mac-client routes I/O to relay
5. Shell exit triggers cleanup

---

## Web UI Features

### Table Stakes (Carry from v1)

Features already validated, must maintain in v2.

| Feature | Status | Notes |
|---------|--------|-------|
| Real-time terminal output | Required | Streaming via WebSocket |
| Keyboard input to terminal | Required | All keys including Ctrl+C, arrows |
| Full terminal emulation | Required | xterm.js handles this |
| Copy/paste support | Required | Browser clipboard API |
| Connection status indicator | Required | Visual feedback essential |
| Session code entry | Required | Simple code input UI |
| Terminal resize | Required | xterm.js fit addon |

### New for v2

Features enabled by universal terminal support.

| Feature | Value Proposition | Complexity | Testable Criteria |
|---------|-------------------|------------|-------------------|
| **Multi-session view** | See all connected terminals, not just one | Medium | Sidebar shows all sessions |
| **Session switching** | Click to switch between terminals | Medium | Clicking session loads that terminal |
| **Session naming/renaming** | Identify terminals meaningfully | Low | User can rename "zsh - ~/code" to "Dev Server" |
| **Session metadata display** | Show PWD, terminal app, shell | Low | Metadata visible in session list |
| **Grid view** | See multiple terminals at once | High | 2x2 or 3x3 grid of terminal previews |
| **Session search/filter** | Find terminal among many | Low | Search box filters session list |
| **Mobile-responsive sidebar** | Usable on iPad | Medium | Sidebar collapses to hamburger on narrow screens |

### Web UI Anti-Features

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **Terminal splits in browser** | Complexity; user has terminal app for this | Multiple sessions, one active view |
| **Tabs in browser UI** | Redundant with browser tabs | Single page, session list in sidebar |
| **Sound notifications** | Annoying, rarely wanted | Visual indicators only |
| **Custom keybindings UI** | Complexity for marginal value | Document defaults, let xterm.js handle |
| **Theme editor** | Scope creep; presets are enough | Offer 3-5 preset themes |

---

## Universal Terminal Support

### What "Universal" Means

| Terminal App | Support Method | Expected Behavior |
|--------------|----------------|-------------------|
| iTerm2 | Shell integration | Auto-connects, full I/O |
| Terminal.app | Shell integration | Auto-connects, full I/O |
| VS Code terminal | Shell integration | Auto-connects, full I/O |
| Warp | Shell integration | Auto-connects, full I/O |
| Kitty | Shell integration | Auto-connects, full I/O |
| Alacritty | Shell integration | Auto-connects, full I/O |
| Hyper | Shell integration | Auto-connects, full I/O |
| JetBrains terminal | Shell integration | Auto-connects, full I/O |
| Zed terminal | Shell integration | Auto-connects, full I/O |

**Key insight:** By integrating at the shell level (not terminal API level), we support ANY terminal app that runs zsh/bash/fish.

### Universal Support Table Stakes

| Feature | Why Expected | Notes |
|---------|--------------|-------|
| Same behavior in all terminals | Consistency is the value prop | No terminal-specific quirks |
| No per-terminal configuration | Setup once, works everywhere | Single .zshrc line |
| No terminal app modification | Users shouldn't install plugins | Pure shell integration |

### What We Lose vs iTerm2-Specific

| v1 Feature | v2 Status | Mitigation |
|------------|-----------|------------|
| Native tab list from iTerm2 | Lost | Sessions ARE tabs in web UI |
| Tab switching in iTerm2 | Lost | Users switch terminals in their app |
| iTerm2-specific metadata | Lost | Shell provides PWD, process name |

**Tradeoff assessment:** Universal support is worth losing iTerm2-specific features. Users want "any terminal" more than "deep iTerm2 integration."

---

## Anti-Features (Global)

Features to deliberately NOT build across all components.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **User accounts** | Single-user tool, adds complexity and security surface | Session codes with expiration |
| **Multi-user collaboration** | Not the use case; adds presence, permissions complexity | Single-user simplicity |
| **Session recording** | Privacy concerns, storage, compliance complexity | Not needed for personal use |
| **File transfer UI** | Scope creep; scp/rsync work fine | Use terminal commands |
| **Local-only mode** | Complicates architecture for edge case | Always use cloud relay |
| **Create new terminals** | We view/control existing, not spawn new | Users create terminals in their app |
| **Terminal multiplexing (splits)** | Terminal apps do this better | One terminal per session |
| **Plugin system** | Massive complexity; focused tool, not platform | Keep it simple |
| **SSH tunneling** | Different use case (infrastructure access) | We're terminal viewing, not SSH |
| **Audit logging** | Enterprise feature with legal implications | Simple connection logs only |
| **Custom keybindings** | Complexity for power users who'll manage | Sensible defaults |

---

## Feature Dependencies

```
                     [Mac-Client Running]
                            |
        +-------------------+-------------------+
        |                                       |
[Shell Integration]                    [Menu Bar UI]
        |                                       |
[Session Registration]              [Session Code Display]
        |                                       |
        +-------------------+-------------------+
                            |
                    [WebSocket Relay]
                            |
                    [Browser Web UI]
                            |
        +-------------------+-------------------+
        |                   |                   |
[Session List]      [Terminal View]     [Session Metadata]
```

**Critical Path for v2:**
1. Mac-client menu bar app (shows session code)
2. Shell integration (connects sessions)
3. Multi-session web UI (view any terminal)

**Can Parallelize:**
- Menu bar polish (QR code, notifications)
- Shell integration for fish
- Web UI session metadata display

---

## MVP Recommendation for v2.0

### Must Have (Table Stakes)

**Menu Bar App:**
1. Status icon with session code
2. Copy session code to clipboard
3. Connection status indicator
4. Session count
5. Quit option

**Shell Integration:**
1. Auto-connect for zsh
2. Silent when mac-client not running
3. No prompt interference
4. Session naming from PWD

**Web UI:**
1. All v1 features (terminal, I/O, resize)
2. Multi-session sidebar
3. Session switching

### Should Have (First Release)

- Start at login option
- Click session in menu to open browser
- Working directory tracking
- Bash support

### Defer to Post-v2.0

- QR code for session code
- Fish shell support
- Command timing/events
- Grid view for multiple terminals
- Session search/filter

---

## Complexity Estimates

| Feature Area | Complexity | Risk | Notes |
|--------------|------------|------|-------|
| Menu bar icon + basic menu | Low | Low | Well-documented in Rust (tray-icon, tauri) |
| Copy to clipboard | Low | Low | Platform APIs available |
| Session code display | Low | Low | Simple text rendering |
| Shell integration (zsh) | Medium | Medium | Hook mechanism understood; edge cases exist |
| Unix socket communication | Medium | Low | Standard IPC pattern |
| Multi-session relay routing | Medium | Medium | Extends v1 architecture |
| Multi-session web UI | Medium | Low | Extends v1 UI |

---

## Sources

**Menu Bar App UX:**
- [Apple Human Interface Guidelines - Menu Bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar)
- [What I Learned Building a Native macOS Menu Bar App](https://dev.to/heocoi/what-i-learned-building-a-native-macos-menu-bar-app-4im6)
- [Creating Status Bar Apps on macOS in Swift](https://www.appcoda.com/macos-status-bar-apps/)

**Shell Integration Patterns:**
- [Kitty Shell Integration](https://sw.kovidgoyal.net/kitty/shell-integration/)
- [VS Code Terminal Shell Integration](https://code.visualstudio.com/docs/terminal/shell-integration)
- [iTerm2 Shell Integration](https://iterm2.com/shell_integration.html)
- [Zsh Hooks Documentation](https://zsh.sourceforge.io/Doc/Release/Functions.html)
- [bash-preexec for Bash](https://github.com/rcaloras/bash-preexec)

**Terminal Sharing Comparisons:**
- [ttyd - Share your terminal over the web](https://tsl0922.github.io/ttyd/)
- [GoTTY - Share terminal as web application](https://github.com/yudai/gotty)
- [Mosh - Mobile Shell](https://mosh.org)

**Session Persistence:**
- [tmux Session Management](https://www.linuxtrainingacademy.com/tmux-tutorial/)
- [Mosh Roaming and Reconnection](https://mosh.org)

**Confidence Assessment:**
| Area | Confidence | Reason |
|------|------------|--------|
| Menu bar features | HIGH | Apple HIG + multiple tutorials + comparable apps |
| Shell integration | HIGH | Documented by kitty/iTerm2/VS Code |
| Universal terminal support | HIGH | Shell-level integration is proven pattern |
| Web UI features | HIGH | Extends validated v1 patterns |
| Complexity estimates | MEDIUM | Based on comparable projects, not this specific stack |

---
*Last updated: 2026-02-06*
