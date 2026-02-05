---
status: resolved
trigger: "After login via code succeeds, no terminal appears on the UI. Relay validation error: cursorBlink expects boolean but receives a number."
created: 2026-02-05T00:00:00Z
updated: 2026-02-05T00:02:00Z
---

## Current Focus

hypothesis: CONFIRMED - cursorBlink type mismatch causes relay to reject config message
test: Traced full data flow from Python bridge -> SessionManager -> Relay -> Browser
expecting: N/A - root cause confirmed, fix applied and verified
next_action: Archive session

## Symptoms

expected: After successful login via code, a terminal UI should render. cursorBlink should be sent as a boolean.
actual: Login completes but no terminal shows. Relay error: INVALID_MESSAGE - cursorBlink expected boolean, received number.
errors: [Connection] Relay error: INVALID_MESSAGE - [{"expected": "boolean", "code": "invalid_type", "path": ["cursorBlink"], "message": "Invalid input: expected boolean, received number"}]
reproduction: Login via code flow, then observe UI - no terminal renders and the error appears in logs.
started: First time setup - terminal has never rendered successfully in this project.

## Eliminated

## Evidence

- timestamp: 2026-02-05T00:00:30Z
  checked: Python bridge iterm-bridge.py line 560
  found: cursorBlink comes from `self._safe_get(profile, "blinking_cursor", False)`. iTerm2's Python API may return an integer (0/1) for blinking_cursor rather than a boolean. Python integers serialize to JSON numbers, not booleans.
  implication: The upstream source of cursorBlink can be a number

- timestamp: 2026-02-05T00:00:35Z
  checked: session-manager.ts line 129
  found: `cursorBlink: config.cursorBlink ?? true` - the nullish coalescing operator (??) only catches null/undefined, NOT falsy numbers like 0. If config.cursorBlink is 0 or 1 (a number), it passes through as-is without conversion to boolean.
  implication: This is the exact point where the type mismatch is introduced

- timestamp: 2026-02-05T00:00:40Z
  checked: protocol.ts line 226
  found: Zod schema requires `cursorBlink: z.boolean()` - strict boolean type
  implication: The relay rejects config messages with numeric cursorBlink values

- timestamp: 2026-02-05T00:00:45Z
  checked: relay/server.ts lines 84-96 (Mac message handler)
  found: When parseMessage fails validation, relay sends error back to Mac and RETURNS without forwarding the message to the browser. The config message is silently dropped from the browser's perspective.
  implication: Browser never receives the config message; terminal uses default options

- timestamp: 2026-02-05T00:00:50Z
  checked: connection.svelte.ts error handler (lines 94-104)
  found: When browser receives an error message from relay, it sets state='disconnected', clears sessionId, and closes the WebSocket. However, the cursorBlink error goes to the Mac, not the browser, so the browser connection is unaffected.
  implication: The config validation error does NOT directly disconnect the browser

- timestamp: 2026-02-05T00:00:55Z
  checked: +page.svelte line 77 and connection/terminal flow
  found: Terminal renders when connectionStore.isConnected AND terminalStore.activeSessionId are both truthy. The joined message sets both. The tab_list (sent by Mac before browser joins) may be lost, but activeSessionId is still set by the joined handler.
  implication: Terminal component should mount, but config failure means it operates with defaults and no iTerm2 theming

## Resolution

root_cause: In mac-client/src/session-manager.ts line 129, `cursorBlink: config.cursorBlink ?? true` passes through the raw value from the Python bridge without type coercion. The iTerm2 Python API returns an integer (0 or 1) for `profile.blinking_cursor`, which Python's json.dumps serializes as a JSON number. The nullish coalescing operator (??) only catches null/undefined, so numbers pass through untouched. The relay's Zod schema requires a strict boolean, causing validation failure. The rejected config message is never forwarded to the browser.
fix: Changed `cursorBlink: config.cursorBlink ?? true` to `cursorBlink: Boolean(config.cursorBlink ?? true)` - wrapping with Boolean() explicitly coerces any numeric or truthy/falsy value to a proper boolean before serialization. Handles all edge cases: 0->false, 1->true, true->true, false->false, undefined->true (default), null->true (default).
verification: TypeScript compilation passes for both mac-client and main project. Boolean() coercion verified for all expected input types (0, 1, true, false, undefined, null) - all produce correct boolean output that satisfies the Zod z.boolean() schema.
files_changed: [mac-client/src/session-manager.ts]
