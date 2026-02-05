# Architecture

**Analysis Date:** 2026-02-04

## Pattern Overview

**Overall:** Event-driven, multi-client WebSocket architecture with real-time bidirectional communication

**Key Characteristics:**
- Dual WebSocket endpoints for separate client types (iTerm2 and browsers)
- Centralized session state management with in-memory storage
- Message-based protocol with strongly typed interfaces
- Request/response pattern where browser inputs are forwarded to iTerm2 clients
- Broadcast model for terminal state updates from iTerm2 to all subscribed browsers

## Layers

**Presentation Layer (Frontend):**
- Purpose: Render terminal UI and accept user input
- Location: `src/routes/`, `src/lib/components/`
- Contains: Svelte pages, components, CSS styling
- Depends on: WebSocket store, notification store
- Used by: End users' browsers

**State Management Layer:**
- Purpose: Manage client-side WebSocket connection and reactive state
- Location: `src/lib/stores/`
- Contains: WebSocket store (`websocket.ts`), notification store (`notifications.ts`)
- Depends on: WebSocket API, browser Notification API
- Used by: Presentation layer components

**Authentication Layer:**
- Purpose: Verify user credentials and manage session tokens
- Location: `src/lib/server/auth.ts`
- Contains: Session token generation/verification, password validation
- Depends on: Environment variables (ITERM_TOKEN, ADMIN_PASSWORD)
- Used by: Server hooks, API routes, WebSocket handlers

**Real-time Communication Layer:**
- Purpose: Route messages between iTerm2 clients and browser clients
- Location: `src/lib/server/websocket.ts`, `server.js`
- Contains: WebSocket server initialization, message handlers, client management
- Depends on: Authentication layer, session management layer
- Used by: HTTP server upgrade events

**Session Management Layer:**
- Purpose: Store and retrieve terminal session state
- Location: `src/lib/server/sessions.ts`
- Contains: Session storage, cursor position tracking, connection status
- Depends on: Types, message protocols
- Used by: WebSocket handler, browser clients

**Notification Detection Layer:**
- Purpose: Detect when iTerm2 is waiting for user approval
- Location: `src/lib/server/notification.ts`, `src/lib/stores/notifications.ts`
- Contains: Pattern matching for approval prompts, notification UI logic
- Depends on: Browser Notification API, audio APIs
- Used by: Frontend components to alert users

**Server Integration Layer:**
- Purpose: Bind SvelteKit to HTTP server and WebSocket handler
- Location: `server.js`
- Contains: Express app setup, WebSocket server integration, API fallbacks
- Depends on: SvelteKit adapter-node, Express, ws library
- Used by: Node.js runtime

## Data Flow

**Terminal Update Flow (iTerm2 → Browser):**

1. iTerm2 client connects to `/ws/iterm` with bearer token authentication
2. iTerm2 sends `ScreenMessage` or `SessionStatusMessage` via WebSocket
3. Server's `handleItermMessage` receives message
4. `sessionManager.updateSession()` or `sessionManager.setConnected()` updates in-memory state
5. `broadcastToBrowsers()` sends `TerminalUpdateMessage` to all subscribed browser clients
6. Browser stores receive message via `wsStore.handleMessage()`
7. `terminals` store updates with new content
8. Components subscribing to `activeTerminal` reactive store re-render

**User Input Flow (Browser → iTerm2):**

1. User types in `InputBox.svelte` and hits Enter
2. `handleSubmit()` calls `wsStore.sendInput(text)`
3. WebSocket store sends `UserInputMessage` to server's `/ws/browser` endpoint
4. Server's `handleBrowserMessage()` receives message
5. `sendToIterm()` finds iTerm2 client with matching `sessionId`
6. Sends `InputMessage` to iTerm2 client
7. iTerm2 application receives input and processes it

**Session State Management:**

1. Session state lives in `sessionManager` Map in `src/lib/server/sessions.ts`
2. Key is `sessionId` (UUID from iTerm2)
3. Value contains: id, name, content, cursorX, cursorY, connected, lastUpdate
4. Browser clients have `activeSessionId` writable store
5. Derived stores (`activeTerminal`, `activeSession`) compute state from active ID

**Authentication State:**

1. Session tokens stored in-memory Map in `src/lib/server/auth.ts`
2. Login creates token via `createSession()`
3. Token set as httpOnly cookie `session`
4. Browser WebSocket upgrades verified by checking cookie
5. Server hooks check token validity before allowing access

## Key Abstractions

**WebSocketManager:**
- Purpose: Abstract WebSocket server setup and client tracking
- Location: `src/lib/server/websocket.ts` (class-based singleton)
- Pattern: Singleton with internal Maps for iTerm and browser clients
- Responsibility: Route messages between client types, manage subscriptions

**SessionManager:**
- Purpose: Store and query terminal session state
- Location: `src/lib/server/sessions.ts` (class-based singleton)
- Pattern: Singleton with Map-based in-memory storage
- Responsibility: CRUD operations on TerminalSession objects

**Message Types:**
- Purpose: Define protocol between all components
- Location: `src/lib/types.ts`
- Pattern: Union types for each message direction
- Types:
  - `ItermToServerMessage` (ScreenMessage, SessionStatusMessage)
  - `ServerToItermMessage` (InputMessage, RequestScreenMessage)
  - `BrowserToServerMessage` (UserInputMessage, SubscribeMessage)
  - `ServerToBrowserMessage` (SessionListMessage, TerminalUpdateMessage, SessionConnectedMessage, SessionDisconnectedMessage)

**Svelte Stores:**
- Purpose: Manage client-side reactive state
- Location: `src/lib/stores/`
- Pattern: Writable stores with derived stores for computed state
- Stores:
  - `wsStore.sessions` (Map<sessionId, SessionInfo>)
  - `wsStore.terminals` (Map<sessionId, TerminalState>)
  - `wsStore.activeSessionId` (string | null)
  - `wsStore.activeTerminal` (derived, current terminal content)
  - `wsStore.activeSession` (derived, current session metadata)
  - `wsStore.sessionList` (derived, sorted array of sessions)

## Entry Points

**Browser Page (/login):**
- Location: `src/routes/login/`
- Triggers: User navigates to `/login` when not authenticated
- Responsibilities:
  - Display password form via `src/routes/login/+page.svelte`
  - Handle form submission via `src/routes/login/+page.server.ts` (SvelteKit actions)
  - Generate session token and set httpOnly cookie
  - Redirect to `/` on success

**Dashboard Page (/):**
- Location: `src/routes/+page.svelte`
- Triggers: User navigates to `/` with valid session cookie
- Responsibilities:
  - Initialize WebSocket store connection on mount
  - Display layout with Terminal, InputBox, SessionList, QuickActions components
  - Manage settings modal state
  - Monitor terminal content for approval notifications

**WebSocket Upgrade (/ws/iterm):**
- Location: `server.js` lines 37-46
- Triggers: HTTP upgrade request to `/ws/iterm` with Bearer token
- Responsibilities:
  - Verify iTerm2 bearer token from auth header
  - Initialize iTerm client and message handlers
  - Track active sessions for this client

**WebSocket Upgrade (/ws/browser):**
- Location: `server.js` lines 51-63
- Triggers: HTTP upgrade request to `/ws/browser` with session cookie
- Responsibilities:
  - Verify session cookie validity
  - Initialize browser client and message handlers
  - Send current session list to new browser client

**API Login (POST /api/login):**
- Location: `src/routes/api/login/+server.ts`
- Triggers: JSON POST request with password
- Responsibilities:
  - Verify password against ADMIN_PASSWORD env var
  - Create session token and set httpOnly cookie
  - Return success/error JSON response

**API Logout (POST /api/logout):**
- Location: `src/routes/api/logout/+server.ts`
- Triggers: POST request with session cookie
- Responsibilities:
  - Delete session token from store
  - Clear session cookie
  - Return success response

## Error Handling

**Strategy:** Try-catch blocks in message handlers with error logging, graceful degradation on connection failures

**Patterns:**

- **Message Parsing:** Try-catch in `handleItermMessage()` and `handleBrowserMessage()` (lines 70-75, 141-146 in websocket.ts) logs errors but continues processing other messages
- **Connection Failures:** WebSocket `onerror` handler logs but does not throw; browser reconnects automatically after 3 seconds
- **Missing Sessions:** `sendToIterm()` logs warning but continues if no client found for session ID
- **Auth Failures:** Return 401 with error response, close socket without crashing server

## Cross-Cutting Concerns

**Logging:**

- Console.log/error in WebSocket handlers with `[WS]` prefix for debugging
- No centralized logging service; logs go to stdout
- Locations: `src/lib/server/websocket.ts` lines 67, 74, 91, etc.

**Validation:**

- Message structure validated via TypeScript types
- JSON parsing wrapped in try-catch
- Session token verified before WebSocket upgrade
- Password compared directly to env variable (no hashing in dev mode)

**Authentication:**

- iTerm2: Bearer token in Authorization header, checked in `verifyItermToken()`
- Browsers: httpOnly session cookie, checked in `verifySession()` and WebSocket upgrade
- Sessions: In-memory Map with 7-day expiration in auth.ts
- Server hooks prevent unauthenticated access to protected routes

**Real-time State Sync:**

- Session list sent to browsers on connection
- Terminal updates broadcast to all subscribed browsers on each iTerm2 screen message
- Subscription model allows browsers to filter updates by sessionId
- No persistence layer; state lost on server restart

---

*Architecture analysis: 2026-02-04*
