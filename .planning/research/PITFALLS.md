# Domain Pitfalls: Rust Terminal Remote Control

**Domain:** Rust macOS menu bar app with terminal I/O relay
**Stack:** Rust, objc2/cacao, tokio, axum, portable-pty, rust-embed
**Researched:** 2026-02-06
**Confidence:** MEDIUM-HIGH (verified with multiple sources)

---

## Critical Pitfalls (Will Break the App)

### Pitfall 1: Main Thread Violations (AppKit/Cocoa)

**What goes wrong:** AppKit requires most UI operations to occur on the main thread. Calling NSStatusBar, NSMenu, or other Cocoa APIs from a Tokio worker thread causes crashes or undefined behavior.

**Why it happens:** Rust async runtimes (Tokio) run tasks on thread pools. Developers naturally call menu bar APIs from async contexts without realizing the threading constraint.

**Consequences:**
- Random crashes with no clear stack trace
- UI freezes while async operations run
- Menu bar icon appears briefly then disappears
- EXC_BAD_ACCESS crashes in Objective-C runtime

**Warning signs:**
- Crashes that only reproduce intermittently
- "Main Thread Checker" errors in Xcode debugger
- Crashes when adding/updating menu items
- Works in debug, crashes in release (timing differences)

**Prevention:**
1. Use objc2's MainThreadMarker for compile-time thread safety guarantees
2. Run Tokio runtime on background threads, NOT the main thread
3. Use channels (mpsc) to send commands from async tasks to main thread
4. Structure app: main thread runs Cocoa event loop, spawns Tokio on background
5. Never hold async locks while calling into Cocoa

**Architecture pattern:**
- Main thread: Cocoa event loop + receive channel
- Background thread: Tokio runtime with async tasks
- Communication: mpsc channels between them

**Phase:** Phase 1 (Menu bar app skeleton) - establish correct threading pattern from the start

**Sources:**
- [objc2 MainThreadMarker docs](https://docs.rs/objc2-foundation/latest/objc2_foundation/struct.MainThreadMarker.html)
- [GUIs and the Main Thread - Rust Forum](https://users.rust-lang.org/t/guis-and-the-main-thread/2863)
- [objc2 GitHub](https://github.com/madsmtm/objc2)

---

### Pitfall 2: PTY Blocking I/O Hangs

**What goes wrong:** PTY read operations are blocking. Reading from a PTY in an async context without spawn_blocking causes the entire Tokio runtime to stall, freezing the application.

**Why it happens:** PTY file descriptors don't support non-blocking I/O in the traditional sense. Developers wrap PTY reads in async functions expecting them to yield, but they block the thread.

**Consequences:**
- Terminal output stops appearing
- WebSocket connections time out
- App becomes completely unresponsive
- High CPU usage in blocked state (or zero, stuck on read)

**Warning signs:**
- Output stops after first command
- Works for small commands, hangs on cat large_file
- Async tasks scheduled but never execute
- App works with println! but not PTY output

**Prevention:**
1. Use tokio::task::spawn_blocking for ALL PTY read/write operations
2. Create dedicated blocking threads for PTY I/O
3. Use portable-pty crate's async-aware patterns
4. Bridge to async with channels:
   - Blocking thread reads PTY -> sends to channel
   - Async task receives from channel -> sends to WebSocket

**Phase:** Phase 2 (Shell integration/PTY work) - design the I/O pipeline correctly

**Sources:**
- [Rust Forum - PTY Output Hangs](https://users.rust-lang.org/t/rust-pty-output-hangs-when-trying-to-read-command-output-in-terminal-emulator/102873)
- [portable-pty documentation](https://docs.rs/portable-pty)
- [developerlife.com - PTY in Rust](https://developerlife.com/2025/08/10/pty-rust-osc-seq/)

---

### Pitfall 3: macOS Notarization with Hardened Runtime

**What goes wrong:** macOS requires both code signing AND notarization for distribution outside the App Store. Hardened runtime blocks certain capabilities unless specific entitlements are granted. Without notarization, Gatekeeper blocks the app entirely on macOS Sequoia.

**Why it happens:** Apple security requirements have tightened. A signed-but-not-notarized app gets "damaged and can't be opened" on modern macOS. The Control-click bypass was removed in Sequoia.

**Consequences:**
- "App is damaged and can't be opened" error
- Gatekeeper quarantine blocks execution completely
- App works in development but fails after distribution
- Users can't install without disabling security (bad UX, support burden)

**Warning signs:**
- Works on your machine, fails for testers
- Works when run from Xcode/Terminal, fails from Finder
- spctl -a -v reports rejection
- App crashes immediately after signing (entitlements wrong)

**Prevention:**
1. Set up Developer ID certificate early ($99/year Apple Developer account)
2. Create proper Entitlements.plist with required capabilities
3. Sign with hardened runtime: codesign --options runtime --entitlements Entitlements.plist -s "Developer ID" app.app
4. Submit to notarization: xcrun notarytool submit app.zip --apple-id ... --wait
5. Staple the ticket: xcrun stapler staple app.app
6. Test full flow BEFORE release: sign, notarize, download, run

**Phase:** Phase 5 (Distribution/packaging) - but set up signing infrastructure in Phase 1

**Sources:**
- [Apple Developer - Resolving Notarization Issues](https://developer.apple.com/documentation/security/resolving-common-notarization-issues)
- [Apple Developer - Developer ID](https://developer.apple.com/developer-id/)
- [Tauri macOS Bundle docs](https://v2.tauri.app/distribute/macos-application-bundle/)

---

### Pitfall 4: TCC Permission Loss on App Updates

**What goes wrong:** macOS TCC (Transparency, Consent, Control) permissions are tied to code signature hash. When the app is updated with a different signature (or rebuilt differently), previously granted permissions (Accessibility, Automation) are revoked.

**Why it happens:** TCC uses code signature to identify apps. Different builds have different signatures unless using consistent Developer ID. Debug vs Release builds have different signatures.

**Consequences:**
- Users must re-grant permissions after every update
- Accessibility features stop working silently post-update
- Terminal control features fail with no error
- Permission dialogs appear repeatedly, annoying users

**Warning signs:**
- Works after fresh install, breaks after update
- Works in development, breaks in production
- Users report "it stopped working" after auto-update
- Console shows TCC denial messages

**Prevention:**
1. Use consistent Developer ID signing from day one (not just for release)
2. Never change bundle identifier between versions
3. Test permission persistence across update scenarios
4. Use SMAppService for login item registration (survives updates better)
5. Document which permissions are needed and provide re-grant instructions

**Phase:** Phase 5 (Distribution) - but plan for this from Phase 1

**Sources:**
- [Tauri Issue #11085 - Permission re-grant after updates](https://github.com/tauri-apps/tauri/issues/11085)
- [tauri-plugin-macos-permissions](https://crates.io/crates/tauri-plugin-macos-permissions)
- [jano.dev - Accessibility Permission](https://jano.dev/apple/macos/swift/2025/01/08/Accessibility-Permission.html)

---

## Moderate Pitfalls (Will Cause Issues)

### Pitfall 5: Shell Integration Conflicts with Oh-My-Zsh/Powerlevel10k

**What goes wrong:** Shell integration scripts using precmd/preexec hooks conflict with popular zsh frameworks. Oh-My-Zsh and Powerlevel10k override or interfere with shell hooks, breaking integration.

**Why it happens:** Multiple tools compete for the same hooks. Oh-My-Zsh initializes hooks, then your integration overwrites them (or vice versa). Powerlevel10k's "instant prompt" adds timing complexity.

**Consequences:**
- Integration works in vanilla zsh but breaks with oh-my-zsh
- Hooks fire inconsistently or not at all
- First prompt works, subsequent prompts don't
- User's existing prompt/theme breaks

**Warning signs:**
- Works for you (vanilla zsh), fails for testers (oh-my-zsh)
- Works on first prompt, fails on subsequent
- precmd_functions array doesn't contain your hook
- Powerlevel10k shows warnings about slow hooks

**Prevention:**
1. Use add-zsh-hook function, not direct assignment
2. Test with oh-my-zsh, Powerlevel10k, and starship configurations
3. Load integration AFTER oh-my-zsh sources (document load order)
4. Provide instructions for .zshrc ordering
5. Use unique function names to avoid collisions

**Recommended shell integration pattern:**
```zsh
# Guard against double-loading
[[ -n "$YOUR_APP_LOADED" ]] && return
export YOUR_APP_LOADED=1

# Guard against non-interactive
[[ -o interactive ]] || return

# Use add-zsh-hook, not direct assignment
autoload -Uz add-zsh-hook

_your_app_precmd() {
  # Fail silently, don't break user's shell
  { your_integration_code } 2>/dev/null || true
}

add-zsh-hook precmd _your_app_precmd
```

**Phase:** Phase 2 (Shell integration) - test extensively with popular configs

**Sources:**
- [VSCode Issue #146587 - Shell integration with oh-my-zsh](https://github.com/microsoft/vscode/issues/146587)
- [Powerlevel10k Issue #1827 - VSCode shell integration](https://github.com/romkatv/powerlevel10k/issues/1827)
- [Oh-My-Zsh Issue #13132 - iTerm2 integration conflict](https://github.com/ohmyzsh/ohmyzsh/issues/13132)

---

### Pitfall 6: SIGWINCH Terminal Resize Not Propagated

**What goes wrong:** When the remote viewer resizes, the PTY window size must be updated via ioctl(TIOCSWINSZ). Failing to handle this causes garbled output, broken TUI apps (vim, htop), and cursor position errors.

**Why it happens:** Resize events are easy to miss. Developers implement resize in the viewer but forget to propagate to the PTY. Signal handlers in Rust are tricky.

**Consequences:**
- vim/nano displays garbled after resize
- TUI apps (htop, top, tmux) render incorrectly
- Text wraps at wrong column
- Cursor position drifts from actual position

**Warning signs:**
- Works for simple commands, breaks for vim/nano
- Resize "works" but output looks wrong
- Works at startup, breaks after first resize
- Works locally, breaks over network (timing)

**Prevention:**
1. When viewer sends resize, call ioctl(TIOCSWINSZ) on PTY master fd
2. Use atomic flag pattern for signal handling
3. Include window size in WebSocket protocol
4. Send resize acknowledgment back to viewer
5. Test with TUI apps: vim, htop, tmux after resize

**Phase:** Phase 3 (WebSocket relay) - include resize in the protocol from the start

**Sources:**
- [SIGWINCH handling patterns](http://www.rkoucha.fr/tech_corner/sigwinch.html)
- [rexpect Issue #10 - resize window on SIGWINCH](https://github.com/rust-cli/rexpect/issues/10)

---

### Pitfall 7: WebSocket Reconnection Not Automatic

**What goes wrong:** WebSocket connections drop (network changes, sleep/wake, proxy timeouts). Neither tokio-tungstenite nor axum provide automatic reconnection. Lost connections mean lost terminal sessions without explicit handling.

**Why it happens:** WebSocket protocol doesn't define reconnection. Libraries implement the protocol, not application-level resilience.

**Consequences:**
- Terminal goes blank after network change
- No error shown to user when connection drops
- Session state lost after reconnection
- Mac waking from sleep = broken session

**Warning signs:**
- Works continuously, fails after network blip
- Works on stable network, fails on wifi
- Laptop sleep/wake breaks connection permanently
- Users report "it just stopped working"

**Prevention:**
1. Implement exponential backoff reconnection on client side
2. Use heartbeat/ping-pong to detect dead connections early
3. Consider ezsockets crate which includes reconnection logic
4. Buffer terminal output during disconnection for replay on reconnect
5. Show connection status indicator in UI
6. Store session ID to reconnect to same PTY

**Phase:** Phase 3 (WebSocket relay) - build reconnection from the start

**Sources:**
- [tokio-tungstenite Issue #101 - Auto reconnect](https://github.com/snapview/tokio-tungstenite/issues/101)
- [axum Discussion #1216 - WebSocket reconnects](https://github.com/tokio-rs/axum/discussions/1216)
- [ezsockets crate](https://crates.io/crates/ezsockets)

---

### Pitfall 8: Binary Size Explosion with Embedded Assets

**What goes wrong:** Embedding web UI assets (HTML, JS, CSS, images) with include_bytes! or rust-embed can balloon binary size. Debug symbols compound the problem. 50MB+ binaries are common without optimization.

**Why it happens:** Rust debug builds include symbols. Web assets add up. Multiple copies of assets in different forms. Compression not applied.

**Consequences:**
- Binary exceeds 50-100MB
- Download/install time frustrates users
- CI/CD slow to upload artifacts
- GitHub releases hit size limits

**Warning signs:**
- Release binary much larger than expected
- Debug binary 10x larger than release
- Adding web assets dramatically increases size
- Users complain about download size

**Prevention:**
1. Enable strip in release profile (Cargo.toml):
   - strip = true
   - opt-level = "z"
   - lto = true
   - codegen-units = 1
2. Minify web assets before embedding
3. Use brotli compression for assets, decompress at runtime
4. Consider lazy loading for non-essential assets

**Phase:** Phase 4 (Embedded web UI) - configure Cargo.toml profiles from start

**Sources:**
- [min-sized-rust guide](https://github.com/johnthagen/min-sized-rust)
- [Kobzol's blog - Making Rust binaries smaller](https://kobzol.github.io/rust/cargo/2024/01/23/making-rust-binaries-smaller-by-default.html)

---

### Pitfall 9: UTF-8 Encoding Mismatches

**What goes wrong:** PTY inherits locale from environment. If LANG/LC_ALL aren't set correctly, UTF-8 characters display as garbage. This especially affects emoji, international characters, and special symbols.

**Why it happens:** macOS may have different default locale. PTY spawned without explicit env vars inherits unpredictable locale.

**Consequences:**
- Emoji appear as question marks or boxes
- International characters garbled
- Git output with non-ASCII broken
- Filenames with special chars unreadable

**Warning signs:**
- Works in your terminal, broken in app
- Works for ASCII, fails for international users
- Emoji show as ??? or boxes

**Prevention:**
1. Set explicit locale when spawning PTY:
   - LANG=en_US.UTF-8
   - LC_ALL=en_US.UTF-8
   - TERM=xterm-256color
2. Use binary WebSocket frames, decode as UTF-8 explicitly
3. Handle invalid UTF-8 gracefully (lossy conversion, not panic)
4. Test with emoji, CJK characters, RTL text

**Phase:** Phase 2 (Shell integration) - set locale in PTY spawn

**Sources:**
- [xterm.js Encoding guide](https://xtermjs.org/docs/guides/encoding/)
- [Baeldung - Linux Terminal Character Encoding](https://www.baeldung.com/linux/terminal-locales-check-character-encoding)

---

### Pitfall 10: Login Item Registration (SMAppService)

**What goes wrong:** Legacy login item APIs are deprecated. macOS Ventura+ requires SMAppService API. Using old APIs fails silently or shows deprecation warnings.

**Why it happens:** Apple regularly deprecates APIs. Most Rust examples online use outdated approaches.

**Consequences:**
- "Open at Login" toggle doesn't persist
- App doesn't start after login despite setting
- Console shows deprecation warnings
- Works on older macOS, fails on Ventura+

**Warning signs:**
- Toggle says "enabled" but app doesn't launch
- Works when tested manually, fails after reboot
- macOS 12 works, macOS 13+ doesn't
- No error returned, just silent failure

**Prevention:**
1. Use smappservice-rs crate for modern API
2. Ensure proper bundle structure with Info.plist
3. Set correct bundle identifier matching your main app
4. Test on macOS Ventura (13+) specifically
5. Handle errors gracefully (user may deny permission)

**Phase:** Phase 5 (Polish/distribution) - but design app bundle structure early

**Sources:**
- [smappservice-rs crate](https://crates.io/crates/smappservice-rs)
- [Hopp blog - Rust app start on login](https://www.gethopp.app/blog/rust-app-start-on-login)
- [theevilbit - SMAppService API](https://theevilbit.github.io/posts/smappservice/)

---

## Minor Pitfalls (Polish Issues)

### Pitfall 11: Universal Binary Creation is Manual

**What goes wrong:** Cargo doesn't natively produce universal (fat) binaries for macOS. You must build for both x86_64 and aarch64 separately, then combine with lipo.

**Consequences:**
- Users report "wrong architecture" errors
- Only M1 OR Intel users can run the app
- Build scripts break in CI

**Prevention:**
- Add both targets: rustup target add x86_64-apple-darwin aarch64-apple-darwin
- Build both, then lipo -create -output

**Phase:** Phase 5 (Distribution) - set up universal binary in CI

**Sources:**
- [Alacritty PR #4683](https://github.com/alacritty/alacritty/pull/4683)
- [Rust Forum - Universal macOS applications](https://users.rust-lang.org/t/universal-macos-applications/51740)

---

### Pitfall 12: Hook Error Cascades in Zsh

**What goes wrong:** An error in any zsh hook function prevents subsequent hooks from running.

**Prevention:**
- Wrap hook code in error handling: { your_code } || true
- Fail silently with logging rather than throwing errors

**Phase:** Phase 2 (Shell integration) - defensive coding in hook script

**Sources:**
- [Zsh Hook documentation](https://zsh.sourceforge.io/Doc/Release/Functions.html)

---

### Pitfall 13: Non-Interactive Shell Sourcing

**What goes wrong:** Shell integration sourced from .zshrc doesn't load in non-interactive shells.

**Prevention:**
- Detect interactive shell: [[ -o interactive ]] || return
- Document that integration is for interactive use only

**Phase:** Phase 2 (Shell integration) - document behavior clearly

**Sources:**
- [Arch Wiki - Zsh](https://wiki.archlinux.org/title/Zsh)

---

### Pitfall 14: WebSocket Backpressure

**What goes wrong:** If the client can't process terminal output as fast as it's produced, buffers grow unbounded.

**Prevention:**
- Implement flow control / backpressure signaling
- Use bounded channels with drop-oldest policy
- Pause PTY reading when WebSocket buffer is full

**Phase:** Phase 3 (WebSocket relay) - design protocol with flow control

**Sources:**
- [WebSocket Backpressure article](https://skylinecodes.substack.com/p/backpressure-in-websocket-streams)

---

### Pitfall 15: rust-embed Debug Mode Behavior

**What goes wrong:** In debug mode with default settings, rust-embed reads files from disk at runtime instead of embedding.

**Prevention:**
- Be aware of this behavior (it's intentional for faster dev builds)
- Test release builds, not just debug
- Or enable debug-embed feature to always embed

**Phase:** Phase 4 (Embedded web UI) - understand rust-embed behavior

**Sources:**
- [rust-embed documentation](https://docs.rs/rust-embed/)

---

## macOS-Specific Concerns

### Code Signing and Distribution Checklist

| Requirement | When Needed | Notes |
|-------------|-------------|-------|
| Developer ID certificate | Any distribution | $99/year Apple Developer Program |
| Code signing | All distribution | codesign -s "Developer ID" |
| Hardened Runtime | Notarization | --options runtime flag |
| Entitlements.plist | Protected APIs | Include with codesign |
| Notarization | Outside App Store | xcrun notarytool submit |
| Stapling | Offline verification | xcrun stapler staple |
| Universal binary | Support Intel + M1 | lipo -create |

### Permission Categories (TCC)

| Permission | TCC Category | Needed For | Programmatic? |
|------------|--------------|------------|---------------|
| Accessibility | kTCCServiceAccessibility | UI control | NO |
| Automation | kTCCServiceAppleEvents | AppleScript | NO |
| Full Disk Access | kTCCServiceSystemPolicyAllFiles | Files | NO |
| Screen Recording | kTCCServiceScreenCapture | Capture | NO |

TCC permissions CANNOT be granted programmatically. User must approve via System Preferences.

### Gatekeeper Bypass (Development Only)

```bash
# Remove quarantine from specific app (safe)
xattr -d com.apple.quarantine /path/to/app.app

# Disable Gatekeeper entirely (risky, temporary only)
sudo spctl --master-disable
```

---

## Shell Integration Edge Cases

### What Can Go Wrong with .zshrc Hook

| Scenario | Problem | Mitigation |
|----------|---------|------------|
| oh-my-zsh loaded after hook | Hooks overwritten | Load integration after oh-my-zsh |
| Powerlevel10k instant prompt | Race condition | Disable instant prompt or load early |
| Syntax errors in .zshrc | Integration never loads | Provide standalone source command |
| Multiple terminal apps | Hook conflicts | Use unique function names |
| Non-login interactive shell | Different startup file | Also add to .zprofile if needed |
| tmux/screen sessions | Different PTY behavior | Test in multiplexed environments |
| SSH sessions | TERM different | Handle various TERM values |
| nvm/pyenv/rbenv | Hook ordering | Document load order |

### Zsh Startup File Order

```
Login + Interactive:    zshenv -> zprofile -> zshrc -> zlogin
Non-login Interactive:  zshenv -> zshrc
Login + Non-interactive: zshenv -> zprofile
Non-login + Non-inter:  zshenv (only)
```

---

## Phase-Specific Warning Summary

| Phase | Primary Pitfalls to Address |
|-------|----------------------------|
| Phase 1: Menu bar skeleton | #1 Main thread, signing infrastructure |
| Phase 2: Shell integration | #2 PTY blocking, #5 hooks, #9 UTF-8, #12-13 |
| Phase 3: WebSocket relay | #6 SIGWINCH, #7 reconnection, #14 backpressure |
| Phase 4: Embedded web UI | #8 binary size, #15 rust-embed |
| Phase 5: Distribution | #3 notarization, #4 TCC, #10 login items, #11 universal |

---

## Pre-Implementation Checklist

**Phase 1 (Menu Bar):**
- [ ] Threading architecture decided (main for Cocoa, background for Tokio)
- [ ] Developer ID certificate obtained (or planned)
- [ ] Bundle identifier chosen and documented

**Phase 2 (Shell Integration):**
- [ ] PTY I/O uses spawn_blocking or dedicated threads
- [ ] Shell hook uses add-zsh-hook pattern
- [ ] Locale (LANG, LC_ALL) set on PTY spawn
- [ ] Tested with oh-my-zsh and Powerlevel10k

**Phase 3 (WebSocket):**
- [ ] Resize protocol includes TIOCSWINSZ update
- [ ] Reconnection with exponential backoff implemented
- [ ] Backpressure / flow control considered
- [ ] Heartbeat for connection health

**Phase 4 (Embedded UI):**
- [ ] Cargo.toml release profile optimized for size
- [ ] Assets minified before embedding
- [ ] Tested both debug and release builds

**Phase 5 (Distribution):**
- [ ] Notarization workflow tested end-to-end
- [ ] Universal binary script working in CI
- [ ] TCC permission flow documented for users
- [ ] SMAppService login item tested on Ventura+

---

## Sources

### Primary Documentation
- [Apple Developer - Code Signing](https://developer.apple.com/developer-id/)
- [Apple Developer - Notarization](https://developer.apple.com/documentation/security/resolving-common-notarization-issues)
- [objc2 crate documentation](https://docs.rs/objc2/)
- [portable-pty crate documentation](https://docs.rs/portable-pty)
- [Zsh Official Documentation](https://zsh.sourceforge.io/Doc/Release/Functions.html)

### Community Resources
- [min-sized-rust guide](https://github.com/johnthagen/min-sized-rust)
- [Tauri macOS distribution docs](https://v2.tauri.app/distribute/macos-application-bundle/)
- [smappservice-rs for login items](https://www.gethopp.app/blog/rust-app-start-on-login)

### Issue Discussions
- [VSCode shell integration issues](https://github.com/microsoft/vscode/issues/146587)
- [Tauri permission loss on updates](https://github.com/tauri-apps/tauri/issues/11085)
- [tokio-tungstenite reconnection](https://github.com/snapview/tokio-tungstenite/issues/101)
- [Rust Forum - PTY hangs](https://users.rust-lang.org/t/rust-pty-output-hangs-when-trying-to-read-command-output-in-terminal-emulator/102873)
