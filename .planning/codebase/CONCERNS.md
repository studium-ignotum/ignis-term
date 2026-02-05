# Codebase Concerns

**Analysis Date:** 2026-02-04

## Tech Debt

**Duplicate WebSocket Implementation:**
- Issue: Both `server.js` and `src/lib/server/websocket.ts` implement WebSocket logic separately. Additionally, `simple-server.js` exists as another alternative implementation. This creates maintenance burden and potential conflicts.
- Files: `/server.js`, `/src/lib/server/websocket.ts`, `/simple-server.js`
- Impact: Inconsistent behavior between implementations, difficult to maintain two code paths, unclear which is actually used
- Fix approach: Remove redundant implementations and consolidate all WebSocket logic into a single source. Either use the Express-based server with properly integrated websocket.ts, or clean up to one clear pattern.

**Unused SESSION_SECRET Configuration:**
- Issue: `.env.example` requires `SESSION_SECRET` but it's never imported or used in auth code. Suggests incomplete migration from planned session encryption.
- Files: `/.env.example`, `/src/lib/server/auth.ts`
- Impact: Configuration noise; security best practice (secrets should be used if defined) not followed
- Fix approach: Either use SESSION_SECRET for session signing/encryption, or remove from configuration entirely

**Type Safety Issues:**
- Issue: `any` types used in Terminal.svelte instead of proper typing
- Files: `/src/lib/components/Terminal.svelte` (lines 8-9: `let terminal: any`, `let fitAddon: any`)
- Impact: Loss of type checking for critical UI component handling terminal rendering
- Fix approach: Type as `Terminal` and `FitAddon` from xterm imports

## Known Bugs

**Cookie Secure Flag Inconsistency:**
- Symptoms: Development mode security differs between form-based and API-based login
- Files: `/src/routes/login/+page.server.ts` (line 23: `secure: false`), `/src/routes/api/login/+server.ts` (line 21: `secure: true`)
- Trigger: Logging in via /login form vs /api/login endpoint
- Workaround: Environment variable not checked properly; development always fails with secure:true on http
- Impact: Form-based login won't work in development when cookies require secure flag

**Silent Error Swallowing:**
- Symptoms: Errors during notification settings load silently fail
- Files: `/src/lib/stores/notifications.ts` (line 47: `catch {}`)
- Trigger: Corrupted localStorage or JSON parse failure
- Impact: Settings lost without user awareness; could degrade notification functionality silently

## Security Considerations

**Plaintext Password Comparison (Critical):**
- Risk: Password is compared directly without hashing despite `bcryptjs` being in dependencies
- Files: `/src/lib/server/auth.ts` (lines 14-18: `verifyPassword` function)
- Current mitigation: Only accessible with environment variable, development-only credentials
- Recommendations: Use `bcryptjs.compare()` for password verification. Hash admin password at startup or store pre-hashed value.

**Hardcoded Weak Default Credentials:**
- Risk: Development .env file contains predictable credentials committed to source
- Files: `/.env` (ITERM_TOKEN=dev-token-12345, ADMIN_PASSWORD=admin)
- Current mitigation: Only development credentials, file should be gitignored
- Recommendations: Move .env to .gitignore immediately. Generate strong defaults. Require explicit configuration for any deployment.

**No CSRF Protection on Form Login:**
- Risk: Form-based login at `/routes/login/+page.server.ts` has no CSRF token validation
- Files: `/src/routes/login/+page.server.ts`
- Current mitigation: SvelteKit provides automatic CSRF protection via request origin check (assumed)
- Recommendations: Verify SvelteKit CSRF middleware is enabled; add explicit CSRF token if needed

**Bearer Token as Simple String Comparison:**
- Risk: iTerm2 authentication relies on simple string matching without any validation
- Files: `/src/lib/server/auth.ts` (lines 7-11: `verifyItermToken`)
- Current mitigation: Token passed as bearer header, requires network access
- Recommendations: Consider token expiration, rotation mechanism, or use signed tokens (JWT)

**Session Store Not Persistent:**
- Risk: All sessions lost on server restart; no recovery mechanism
- Files: `/src/lib/server/auth.ts` (lines 28-34: sessions Map)
- Current mitigation: None - development only
- Recommendations: For production: use Redis, database, or encrypted cookie sessions

**No Input Validation on Terminal Input:**
- Risk: User input sent directly to iTerm2 without validation
- Files: `/src/lib/stores/websocket.ts` (lines 162-170: `handleBrowserMessage`)
- Current mitigation: XTerm.js interface prevents obvious injection, but raw text passed through
- Recommendations: Validate input length, rate limit, sanitize control characters if needed

## Performance Bottlenecks

**Unbounded Terminal Content Storage:**
- Problem: Terminal content for each session stored completely in memory with no size limits
- Files: `/src/lib/server/sessions.ts` (lines 16-26: `updateSession` always stores full content)
- Cause: Using in-memory Map with no eviction; terminal scrollback can grow very large
- Improvement path: Implement circular buffer for terminal content, cap size (e.g., last 100KB), or use temporary file storage

**Full Content Clear & Rewrite on Updates:**
- Problem: Every terminal update clears and rewrites entire xterm.js content
- Files: `/src/lib/components/Terminal.svelte` (lines 76-84: `$effect` that calls `terminal.clear()` then `terminal.write()`)
- Cause: Simplistic approach; should incrementally append new content instead
- Improvement path: Maintain differential updates, only write new lines, or use xterm.js write() correctly for incremental updates

**Session Map Grows Unbounded:**
- Problem: Browser and iTerm client tracking objects grow indefinitely
- Files: `/src/lib/server/websocket.ts` (lines 24-25: `itermClients` Map, `browserClients` Set)
- Cause: No cleanup of disconnected clients; orphaned sessions never removed
- Improvement path: Add reaper task to clean disconnected clients; track connection timestamps; add max session limits

**No Message Rate Limiting:**
- Problem: WebSocket message handlers have no rate limiting
- Files: `/src/lib/server/websocket.ts` (lines 69-76, 140-147)
- Cause: Any client can spam messages indefinitely
- Improvement path: Add per-client rate limiter; track messages per second; implement backpressure

## Fragile Areas

**WebSocket Upgrade Handler Assumes Headers Exist:**
- Files: `/src/lib/server/websocket.ts` (line 31: `new URL(request.url || '', ...)`)
- Why fragile: Malformed requests could cause crashes; no validation of request object properties
- Safe modification: Always validate `request.url`, `request.headers` before use; add defensive checks
- Test coverage: No tests for malformed WebSocket upgrade requests

**Session Selection Auto-Fallback with Side Effects:**
- Files: `/src/lib/stores/websocket.ts` (lines 97-99, 127-129: auto-select session)
- Why fragile: Automatically selecting a session can confuse user expectations; multiple message handlers trigger same logic
- Safe modification: Use single source of truth for session selection logic; trigger once not in multiple places
- Test coverage: No tests for session selection behavior

**Hardcoded Cookie Parsing:**
- Files: `/src/lib/server/websocket.ts` (lines 216-225: `parseCookies` function)
- Why fragile: Doesn't handle edge cases (empty values, malformed cookies, special characters)
- Safe modification: Use established cookie parsing library or add comprehensive validation
- Test coverage: No unit tests for cookie parsing edge cases

**Circular Dependency Risk:**
- Files: Both `/src/hooks.server.ts` and `/server.js` attempt WebSocket initialization
- Why fragile: Unclear initialization order; hooks.server.ts references wsManager but doesn't explicitly initialize it
- Safe modification: Centralize WebSocket initialization in one place; make dependency explicit
- Test coverage: No integration tests verifying initialization order

**Terminal Content as Proxy for State:**
- Files: `/src/lib/stores/notifications.ts` (lines 106-143: `checkAndNotify`)
- Why fragile: Notification detection relies on regex matching terminal content; false positives likely
- Safe modification: Use explicit flags in messages rather than content analysis; add explicit approval request type
- Test coverage: No tests for approval pattern detection; patterns are brittle regexes

## Scaling Limits

**In-Memory Session Storage:**
- Current capacity: As many sessions as system memory allows; typical ~1KB per session in Map
- Limit: On 1GB server, roughly 1M sessions possible before memory exhaustion
- Scaling path: Switch to Redis for distributed sessions, add session limits per user, implement automatic cleanup

**Unbounded Browser Client Tracking:**
- Current capacity: Each browser client ~200 bytes; Set grows with concurrent connections
- Limit: 10,000+ concurrent browsers possible on modern server before resource exhaustion
- Scaling path: Add max connection limits, implement connection pooling, horizontal scaling with load balancer

**Terminal Content In-Memory Buffer:**
- Current capacity: Default 10,000 line scrollback set in Terminal.svelte (line 49)
- Limit: With 100+ terminals × 10,000 lines each, can hit GB quickly
- Scaling path: Implement circular buffer, database-backed content, or write to disk

**Single Server Process:**
- Current capacity: All sessions/clients tied to one Node process
- Limit: Single process bottleneck; cannot distribute load
- Scaling path: Use horizontal scaling with sticky sessions, Redis session store, or Kubernetes

## Dependencies at Risk

**bcryptjs Unused (Critical):**
- Risk: `bcryptjs@3.0.3` in dependencies but plaintext password comparison used instead
- Impact: Security vulnerability; library overhead without benefit
- Migration plan: Either use bcryptjs for password hashing (recommended), or remove from dependencies

**XTerm.js Heavy UI Dependency:**
- Risk: Large terminal emulation library in browser; adds significant JS bundle size
- Impact: Slower page loads; may be overkill if only displaying read-only terminal
- Migration plan: Consider lightweight alternatives if read-only (terminal-kit, simple div), or evaluate if terminal interactivity needed for all use cases

**ws Library Version:**
- Risk: `ws@8.18.0` used; check for security advisories
- Impact: Potential unpatched WebSocket vulnerabilities
- Migration plan: Run `npm audit` regularly; update ws if vulnerabilities found

## Missing Critical Features

**Session Revocation/Logout:**
- Problem: No way to invalidate a session token except wait for 7-day expiration
- Blocks: Forced logout, compromised token revocation, per-session permissions
- Impact: Users can't immediately revoke access; security incident recovery limited

**Audit Logging:**
- Problem: No logging of authentication, input, or administrative actions
- Blocks: Compliance, security investigation, usage tracking
- Impact: No record of who did what; impossible to detect unauthorized access after the fact

**Connection State Recovery:**
- Problem: Disconnected browser loses all state; must reconnect and reselect session
- Blocks: Seamless reconnection, mobile network reliability
- Impact: Poor user experience on unstable networks

**Graceful Shutdown:**
- Problem: Server has no shutdown handler to close connections cleanly
- Blocks: Zero-downtime deployments, clean server restarts
- Impact: Abrupt WebSocket disconnects for all clients; lost in-flight messages

**Rate Limiting & DDoS Protection:**
- Problem: No rate limiting on login attempts, WebSocket messages, or input
- Blocks: Protection against brute force, message bombing, resource exhaustion
- Impact: Vulnerable to attacks; one malicious client can degrade service for all

**Message Validation & Sanitization:**
- Problem: WebSocket messages parsed without schema validation
- Blocks: Preventing malformed messages, injection attacks
- Impact: Unexpected message format could crash handler or execute arbitrary code

## Test Coverage Gaps

**No Unit Tests for Authentication:**
- What's not tested: Password verification, session creation/validation, token generation
- Files: `/src/lib/server/auth.ts`
- Risk: Security logic completely untested; regressions in auth could go unnoticed
- Priority: High

**No WebSocket Protocol Tests:**
- What's not tested: Message handling, session subscription, reconnection logic, edge cases
- Files: `/src/lib/server/websocket.ts`, `/src/lib/stores/websocket.ts`
- Risk: Core communication layer untested; multi-client scenarios unknown
- Priority: High

**No Component Tests:**
- What's not tested: Terminal rendering, session selection, notification patterns
- Files: `/src/lib/components/Terminal.svelte`, all Svelte components in `/src/lib/components/`
- Risk: UI regressions undetected; approval pattern detection completely untested
- Priority: Medium

**No Integration Tests:**
- What's not tested: Full login flow, browser-to-iTerm communication, session persistence
- Files: All routes and server logic
- Risk: End-to-end scenarios could fail despite unit tests passing
- Priority: Medium

**No E2E Tests:**
- What's not tested: User workflows (login → connect → send input → receive output)
- Risk: Real-world usage patterns unknown; deployment issues could occur
- Priority: Low (consider as product matures)

---

*Concerns audit: 2026-02-04*
