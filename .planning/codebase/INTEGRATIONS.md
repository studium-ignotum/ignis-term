# External Integrations

**Analysis Date:** 2026-02-04

## APIs & External Services

**iTunes Terminal (iTerm2) Integration:**
- Service: iTerm2 terminal emulator (direct client connection via WebSocket)
  - SDK/Client: WebSocket client (native, no SDK)
  - Auth: Bearer token via `Authorization` header
  - Connection: WebSocket at `/ws/iterm`

**No Third-Party APIs:** The system does not integrate with external SaaS platforms, cloud services, or third-party APIs. All integrations are internal or between iTerm2 and the web dashboard.

## Data Storage

**Databases:**
- None - Not applicable

**Session Storage:**
- In-Memory Map (development/simple deployment)
  - Location: `src/lib/server/sessions.ts` (SessionManager class)
  - Implementation: JavaScript Map data structure
  - Persistence: None (sessions lost on server restart)
  - Production Note: Requires replacement with persistent store (Redis, database)

**Terminal State Storage:**
- In-Memory Map
  - Location: `src/lib/server/sessions.ts` (SessionManager class)
  - Stored data: Terminal content, cursor position, session metadata
  - Persistence: None (state lost on server restart)

**File Storage:**
- Local filesystem only - Not applicable for persistent data
- Static files served from `/static` directory

**Caching:**
- None configured

## Authentication & Identity

**Auth Provider:**
- Custom implementation (no third-party auth service)

**Implementation Details:**
- Location: `src/lib/server/auth.ts`
- Password-based authentication:
  - Admin password stored as plaintext in `ADMIN_PASSWORD` env var (not hashed in current implementation)
  - Comparison at: `src/lib/server/auth.ts` (line 17) - direct string comparison
  - Session generation: Random 32-byte token generated via `crypto.getRandomValues()`
- Session management:
  - Location: `src/lib/server/auth.ts`
  - Cookie-based with httpOnly, secure, and sameSite=strict flags
  - Expiration: 7 days
  - Storage: In-memory Map in `src/lib/server/auth.ts` (line 28)

**Token/Cookie:**
- Cookie name: `session`
- Secure flags: httpOnly=true, sameSite=strict
- Transport: HTTPS in production (secure=true when not in dev)
- Verification: `verifySession()` checks Map existence and expiration time

## Monitoring & Observability

**Error Tracking:**
- None detected

**Logs:**
- Console logging only
  - WebSocket connection/disconnection events: `src/lib/server/websocket.ts`
  - Message parsing errors: Caught and logged to console
  - Example: `console.log('[WS] iTerm2 client connected')`

**No APM, metrics, or error reporting services configured.**

## CI/CD & Deployment

**Hosting:**
- Node.js server via `server.js` or `simple-server.js`
- Intended deployment: Any Node.js-compatible hosting (Heroku, VPS, Docker, etc.)

**CI Pipeline:**
- None detected - No GitHub Actions, GitLab CI, or other CI/CD configuration

**Deployment Method:**
- SvelteKit adapter-node builds to `build/` directory
- Production start: `node server.js` (or alternative Node servers)
- Entry point: `src/routes/+page.svelte` (main dashboard)
- Static assets: Served from `build/client` and `/static`

## Environment Configuration

**Required Environment Variables:**
- `ITERM_TOKEN` - Bearer token for iTerm2 WebSocket authentication
- `ADMIN_PASSWORD` - Dashboard login password
- `SESSION_SECRET` - Defined in example but currently unused (no-op)

**Optional Environment Variables:**
- `NODE_ENV` - Controls secure cookie flag (affects HTTPS requirement)
- `PORT` - Server port (default: 3000)

**Secrets Location:**
- `.env` file (not committed, git-ignored)
- Environment variables injected at runtime
- No secrets management service (AWS Secrets Manager, Vault, etc.)

## Webhooks & Callbacks

**Incoming:**
- None configured

**Outgoing:**
- None configured

## WebSocket Protocol

**Browser to Server (`/ws/browser`):**
- Authentication: Session cookie verification
- Messages:
  - `subscribe` - Subscribe to terminal session updates (sessionId optional for all sessions)
  - `user_input` - Send terminal input to iTerm2 client
- Handlers: `src/lib/server/websocket.ts` lines 155-181

**iTerm2 to Server (`/ws/iterm`):**
- Authentication: Bearer token in Authorization header
- Messages:
  - `screen` - Terminal screen content update
  - `session_status` - Session connection status change
- Handlers: `src/lib/server/websocket.ts` lines 95-126

**Server Broadcasts to Browsers:**
- `terminal_update` - Terminal content change
- `session_list` - Current session list on connect
- `session_connected` - Session became available
- `session_disconnected` - Session unavailable

## Architecture

**Client-Side (Browser):**
- WebSocket store: `src/lib/stores/websocket.ts`
- Authentication: Password-based login at `/login` route
- Session storage: Cookies (managed by SvelteKit)

**Server-Side:**
- WebSocket manager: `src/lib/server/websocket.ts`
- Session manager: `src/lib/server/sessions.ts`
- Auth verification: `src/lib/server/auth.ts`
- API routes: `/routes/api/login`, `/routes/api/logout`

---

*Integration audit: 2026-02-04*
