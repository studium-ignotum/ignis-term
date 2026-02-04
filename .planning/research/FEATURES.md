# Feature Landscape: Remote Terminal Control Web App

**Domain:** Web-based remote terminal access (iTerm2 relay)
**Researched:** 2026-02-04
**Confidence:** MEDIUM (based on training data, web verification unavailable)

## Competitive Landscape Context

This analysis draws from established products in the remote terminal space:

| Product | Focus | Relevance |
|---------|-------|-----------|
| ttyd | Simple web terminal sharing | Direct competitor pattern |
| gotty | Share terminal as web app | Direct competitor pattern |
| Wetty | Web-based TTY over HTTP | Similar architecture |
| Teleport | Enterprise SSH web access | Feature reference (overpowered) |
| Apache Guacamole | Clientless remote desktop | Feature reference |
| Cloud Shell (AWS/GCP/Azure) | Browser-based CLI | UX reference |
| Upterm | Terminal session sharing | Collaboration features |

**Your differentiator:** Native iTerm2 integration (tabs, existing sessions) vs. spawning new shells.

---

## Table Stakes

Features users expect. Missing = product feels broken or incomplete.

| Feature | Why Expected | Complexity | Dependencies | Notes |
|---------|--------------|------------|--------------|-------|
| **Real-time bidirectional I/O** | Core purpose - typing must appear instantly (<100ms perceived) | Medium | WebSocket infrastructure | Latency is the #1 UX killer |
| **Full terminal emulation** | Users expect colors, cursor positioning, vim/tmux to work | Low | xterm.js handles this | VT100/xterm escape sequences |
| **Copy/paste support** | Universal expectation from any text interface | Low | Browser clipboard API | Clipboard permissions vary by browser |
| **Special key handling** | Ctrl+C, arrow keys, Tab, etc. must work correctly | Medium | xterm.js + key mapping | Browser intercepts some keys (Ctrl+W, Ctrl+N) |
| **Responsive terminal sizing** | Terminal must resize with browser window | Low | xterm.js fit addon | Send SIGWINCH equivalent to remote |
| **Secure connection** | Users won't trust plaintext for terminal access | Low | TLS on relay | WSS not WS |
| **Session codes/auth** | Prevent unauthorized access to terminal | Low | Already planned | Session code approach is good for single-user |
| **Connection status indicator** | User must know if connected/disconnected | Low | WebSocket state | Visual indicator required |
| **Reconnection handling** | Network blips shouldn't lose session | Medium | Session persistence | Grace period before session death |
| **Unicode/UTF-8 support** | Modern terminals display emoji, international chars | Low | xterm.js handles this | Ensure encoding consistency end-to-end |
| **256 color support** | Syntax highlighting, prompts expect colors | Low | xterm.js default | TrueColor (16M) is bonus, not required |

### Table Stakes Priority Order

1. Real-time I/O (without this, product is unusable)
2. Full terminal emulation (vim/tmux must work)
3. Special key handling (keyboard is primary input)
4. Secure connection (trust requirement)
5. Copy/paste (workflow integration)
6. Everything else

---

## Differentiators

Features that set your product apart. Not expected, but valued.

| Feature | Value Proposition | Complexity | Dependencies | Notes |
|---------|-------------------|------------|--------------|-------|
| **iTerm2 tab visibility** | See ALL open tabs, not just one session | Medium | iTerm2 scripting API | **Your key differentiator** |
| **Tab switching from browser** | Control which iTerm2 tab is active | Medium | iTerm2 scripting API | Makes it a true remote control |
| **Session persistence across disconnects** | Refresh browser, session survives | Medium | Server-side state | Huge UX improvement |
| **Scrollback buffer access** | View command history, not just current screen | Medium | Buffer sync strategy | How much to send? On-demand vs stream |
| **Clickable URLs** | Links in terminal output open in browser | Low | xterm.js web-links addon | Quality of life |
| **Search in scrollback** | Find text in terminal history | Low | xterm.js search addon | Power user feature |
| **Custom themes** | Match user's terminal aesthetic | Low | xterm.js theming | Store preference locally |
| **Mobile-friendly UI** | Usable on phone/tablet in emergency | High | Touch keyboard, gestures | Nice-to-have, not core |
| **Low-latency optimization** | Feels local even over internet | High | Binary protocol, compression | Differentiates from laggy alternatives |
| **Keyboard shortcut parity** | Cmd+K to clear, etc. work | Medium | Key mapping layer | iTerm2 users expect these |

### Differentiator Priority (Recommended)

**Phase 1 - Core Differentiators:**
1. iTerm2 tab visibility (this IS your product)
2. Tab switching from browser
3. Session persistence across disconnects

**Phase 2 - Polish Differentiators:**
4. Clickable URLs (low effort, high value)
5. Search in scrollback (low effort)
6. Keyboard shortcut parity

**Phase 3 - Advanced:**
7. Custom themes
8. Low-latency optimization
9. Mobile support

---

## Anti-Features

Features to explicitly NOT build. Common mistakes or scope creep traps.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **User accounts/auth system** | Project is single-user. Accounts add complexity, security surface, and maintenance burden | Session codes with expiration. Stateless auth. |
| **Session recording/replay** | Enterprise feature. Adds storage, privacy concerns, complexity. Not needed for personal use. | If needed later, add as separate concern |
| **File transfer/SFTP** | Scope creep. scp/rsync work fine in terminal. Building file UI is huge undertaking. | Just use terminal commands |
| **Multi-user collaboration** | Not the use case. Adds presence, cursor sync, permissions. | Single-user simplicity is a feature |
| **Terminal multiplexing (splits)** | iTerm2 already does this. Don't rebuild. | Leverage iTerm2's existing tab/split support |
| **Shell/environment management** | iTerm2 handles this. You're a viewer, not a shell manager. | Pass through to iTerm2 |
| **Audit logging** | Enterprise/compliance feature. Storage, retention, legal implications. | Simple connection logs if needed |
| **Custom keybindings UI** | Complexity for marginal value. Power users can handle defaults. | Sensible defaults, document them |
| **Browser notifications** | Annoying, permission fatigue, marginal value for terminal use | Status visible in tab title is sufficient |
| **Plugin/extension system** | Massive complexity. You're building a focused tool, not a platform. | Keep it simple |

### The "Not Yet" List

These aren't bad features, just wrong for v1:

- Session recording (maybe v2 if users want it)
- Mobile-optimized UI (works on mobile, but not optimized)
- Offline mode / PWA (maybe later)
- Multiple relay support (one machine for now)

---

## Feature Dependencies

```
                    [WebSocket Connection]
                           |
                    [TLS/Secure Layer]
                           |
            +--------------+--------------+
            |              |              |
     [Terminal I/O]  [Tab Listing]  [Tab Control]
            |              |              |
     [xterm.js render]     +------+------+
            |                     |
     +------+------+      [iTerm2 Scripting API]
     |             |
[Copy/Paste]  [Key Handling]
     |             |
     +------+------+
            |
     [Search/Links] (addons, can be added independently)
```

**Critical Path:**
1. WebSocket relay (everything depends on this)
2. Terminal I/O (core functionality)
3. iTerm2 tab integration (key differentiator)

**Independent Features (can parallelize):**
- xterm.js addons (search, links, themes)
- Scrollback sync
- Reconnection handling

---

## MVP Recommendation

For MVP, prioritize:

**Must Have (Table Stakes Subset):**
1. Real-time bidirectional I/O
2. Full terminal emulation (xterm.js)
3. Copy/paste support
4. Special key handling
5. Secure connection (WSS)
6. Session code auth
7. Connection status indicator
8. Basic reconnection

**Must Have (Core Differentiator):**
9. View iTerm2 tabs list
10. Switch between iTerm2 tabs

**Defer to Post-MVP:**
- Scrollback buffer sync beyond current screen
- Search in scrollback
- Clickable URLs
- Custom themes
- Mobile optimization
- Keyboard shortcut parity (beyond basics)

---

## Complexity Estimates

| Feature Category | Complexity | Effort Estimate | Risk |
|-----------------|------------|-----------------|------|
| WebSocket relay | Medium | 2-3 days | Medium (deployment) |
| xterm.js integration | Low | 1 day | Low (well-documented) |
| iTerm2 scripting bridge | Medium-High | 3-5 days | High (API quirks) |
| Session code auth | Low | 0.5 day | Low |
| Reconnection handling | Medium | 1-2 days | Medium (edge cases) |
| Tab listing/switching | Medium | 2-3 days | Medium (iTerm2 dependent) |

**Highest Risk Areas:**
1. iTerm2 scripting API - May have undocumented limitations
2. Latency optimization - May need iteration
3. Key handling edge cases - Browser varies

---

## Sources and Confidence

| Claim | Confidence | Basis |
|-------|------------|-------|
| xterm.js capabilities | HIGH | Well-established library, extensive documentation |
| Table stakes features | HIGH | Common across all products in space |
| Differentiator value | MEDIUM | Based on product landscape, not user research |
| Complexity estimates | MEDIUM | Based on similar projects, not this specific stack |
| Anti-features | HIGH | Clearly out of scope per project requirements |

**Gaps requiring validation:**
- iTerm2 scripting API capabilities and limitations
- Actual latency characteristics of relay architecture
- Browser-specific key handling quirks

---

## Summary for Roadmap

**Build order recommendation:**

1. **Phase 1: Core Terminal** - WebSocket relay, xterm.js, basic auth
   - All table stakes except polish items
   - Get "it works" baseline

2. **Phase 2: iTerm2 Integration** - Tab listing, tab switching
   - Core differentiator that justifies the product
   - Depends on Phase 1 stability

3. **Phase 3: Polish** - Reconnection, addons, themes
   - Quality of life improvements
   - Can be iterative

**Red flags that indicate scope creep:**
- "What about multiple users?"
- "Can we add file transfer?"
- "Should we support other terminals besides iTerm2?"
- "Let's add session recording"

Stay focused. A great single-user iTerm2 remote is better than a mediocre general-purpose terminal.
