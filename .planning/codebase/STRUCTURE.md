# Codebase Structure

**Analysis Date:** 2026-02-04

## Directory Layout

```
/Users/nat/Desktop/claude-code-remote/
├── src/                           # SvelteKit application source
│   ├── routes/                    # File-based routing
│   │   ├── +layout.svelte         # Root layout wrapper
│   │   ├── +page.svelte           # Dashboard (/)
│   │   ├── login/
│   │   │   ├── +page.svelte       # Login form UI
│   │   │   └── +page.server.ts    # Login form actions
│   │   └── api/
│   │       ├── login/
│   │       │   └── +server.ts     # JSON login endpoint (POST)
│   │       └── logout/
│   │           └── +server.ts     # JSON logout endpoint (POST)
│   ├── lib/                       # Shared library code
│   │   ├── components/            # Reusable Svelte components
│   │   │   ├── Terminal.svelte    # xterm.js terminal renderer
│   │   │   ├── InputBox.svelte    # User input form
│   │   │   ├── SessionList.svelte # Session sidebar
│   │   │   ├── StatusBar.svelte   # Top status bar
│   │   │   ├── QuickActions.svelte# Quick action buttons
│   │   │   └── Settings.svelte    # Settings modal
│   │   ├── server/                # Server-only modules
│   │   │   ├── auth.ts            # Authentication & session logic
│   │   │   ├── websocket.ts       # WebSocket server implementation
│   │   │   ├── sessions.ts        # Session state management
│   │   │   └── notification.ts    # Approval detection patterns
│   │   ├── stores/                # Svelte stores (client-side state)
│   │   │   ├── websocket.ts       # WebSocket connection & state
│   │   │   └── notifications.ts   # Browser notification settings
│   │   └── types.ts               # TypeScript interfaces & types
│   ├── app.d.ts                   # Global TypeScript declarations
│   ├── app.html                   # HTML template
│   ├── app.css                    # Global styles
│   └── hooks.server.ts            # SvelteKit server hooks
├── build/                         # Production build output (generated)
├── .svelte-kit/                   # SvelteKit generated files
├── static/                        # Static assets (favicon, images)
├── server.js                      # Express server wrapper
├── svelte.config.js               # SvelteKit config
├── vite.config.ts                 # Vite build config
├── tsconfig.json                  # TypeScript config
├── package.json                   # npm dependencies and scripts
└── .env                           # Environment variables (dev)
```

## Directory Purposes

**src/routes/:**
- Purpose: File-based routing following SvelteKit conventions
- Contains: Page components, API endpoints, layout wrappers
- Key files: `+page.svelte`, `+page.server.ts`, `+server.ts`

**src/lib/components/:**
- Purpose: Reusable UI components
- Contains: 6 Svelte components for terminal display, input, session management
- Exports: Each .svelte file is a standalone component

**src/lib/server/:**
- Purpose: Server-only business logic, not exposed to browser
- Contains: Authentication, WebSocket management, session state, notification detection
- Imports: Only imported in server-side files (+server.ts, hooks.server.ts)
- Pattern: Singleton exports (wsManager, sessionManager)

**src/lib/stores/:**
- Purpose: Svelte writable/derived stores for reactive client state
- Contains: WebSocket connection manager, notification preferences
- Pattern: Export single store instance per file
- Used by: Components via $store reactive binding

**build/:**
- Purpose: Production build output
- Auto-generated: Yes (created by `npm run build`)
- Committed: No (in .gitignore)
- Contents: Compiled JavaScript and assets

**.svelte-kit/:**
- Purpose: SvelteKit framework-generated files and cache
- Auto-generated: Yes
- Committed: No (should be in .gitignore)
- Contents: Type definitions, adapter output, manifest

## Key File Locations

**Entry Points:**
- `src/routes/+page.svelte`: Main dashboard page (requires authentication)
- `src/routes/login/+page.svelte`: Login form page (public)
- `src/routes/api/login/+server.ts`: Login API endpoint
- `server.js`: Node.js server initialization, WebSocket server setup
- `src/hooks.server.ts`: SvelteKit server-side middleware

**Configuration:**
- `svelte.config.js`: SvelteKit configuration (adapter: node, CSRF settings)
- `vite.config.ts`: Vite bundler configuration
- `tsconfig.json`: TypeScript compiler settings
- `package.json`: Dependencies and npm scripts

**Core Logic:**
- `src/lib/server/websocket.ts`: WebSocket message routing and client management
- `src/lib/server/sessions.ts`: Terminal session state storage
- `src/lib/server/auth.ts`: Authentication and session tokens
- `src/lib/stores/websocket.ts`: Client-side WebSocket connection and reactive state

**UI Components:**
- `src/lib/components/Terminal.svelte`: xterm.js terminal with resize handling
- `src/lib/components/InputBox.svelte`: Single/multi-line input form
- `src/lib/components/SessionList.svelte`: Sidebar list of active sessions
- `src/lib/components/StatusBar.svelte`: Top status bar with connection indicator
- `src/lib/components/QuickActions.svelte`: Action buttons
- `src/lib/components/Settings.svelte`: Modal for notification settings

**Types & Interfaces:**
- `src/lib/types.ts`: All message types and session interfaces
- `src/app.d.ts`: SvelteKit global App namespace (Locals, PageData)

## Naming Conventions

**Files:**

- Svelte components: PascalCase (e.g., `Terminal.svelte`, `InputBox.svelte`)
- Server modules: camelCase (e.g., `websocket.ts`, `sessions.ts`)
- Routes: +layout.svelte, +page.svelte, +server.ts (SvelteKit convention)
- API files: Named by endpoint path (e.g., `routes/api/login/+server.ts` for POST /api/login)

**Directories:**

- lib subdirs: lowercase plural when containing multiple items (e.g., `components/`, `stores/`, `server/`)
- Routes: lowercase, hyphenated for multi-word slugs (e.g., `routes/login/`)
- Feature-based: NOT used; instead uses SvelteKit's file-based routing

**Functions:**

- Handlers: camelCase, prefixed with purpose (e.g., `handleItermMessage()`, `handleBrowserConnection()`)
- Factories: camelCase, prefixed with `create` (e.g., `createWebSocketStore()`, `createNotificationStore()`)
- Stores: camelCase with `Store` suffix (e.g., `wsStore`, `notificationStore`)
- Messages: PascalCase (e.g., `ScreenMessage`, `UserInputMessage`)

**Variables:**

- Constants: UPPER_SNAKE_CASE (e.g., `ITERM_TOKEN`, `ADMIN_PASSWORD`, `APPROVAL_PATTERNS`)
- Client-side state: camelCase (e.g., `activeTerminal`, `activeSessionId`)
- Server state: camelCase (e.g., `itermClients`, `browserClients`)

**Types:**

- Interfaces: PascalCase (e.g., `ItermClient`, `BrowserClient`, `SessionInfo`)
- Union types: PascalCase, descriptive (e.g., `ItermToServerMessage`)
- Enums: Not used; prefer union of string literals

## Where to Add New Code

**New Feature:**

- Primary code: `src/lib/server/` for logic, `src/lib/stores/` for client state
- Tests: Would go in `src/lib/` folder with `.test.ts` suffix (no test folder currently)
- Routes: Add new directory in `src/routes/` following SvelteKit pattern
- Components: Add `.svelte` file to `src/lib/components/`

**New Component/Module:**

- UI Component: Create `src/lib/components/NewComponent.svelte`
- Server logic: Create `src/lib/server/newModule.ts` (export singleton instances)
- Client store: Create `src/lib/stores/newStore.ts` (export single store instance)
- API endpoint: Create `src/routes/api/endpoint/+server.ts`

**Utilities:**

- Shared helpers: Create `src/lib/server/helpers.ts` or `src/lib/utils.ts`
- Client utilities: Create `src/lib/utils.ts` or feature-specific files
- Type utilities: Add to `src/lib/types.ts` or create `src/lib/types/` subdirectory

**Message Types:**

- All message protocols go in `src/lib/types.ts`
- Organize by direction: `ItermToServerMessage`, `ServerToItermMessage`, `BrowserToServerMessage`, `ServerToBrowserMessage`
- Use union types for multiple message kinds

**Authentication & Authorization:**

- Session logic: `src/lib/server/auth.ts`
- Middleware hooks: `src/hooks.server.ts`
- Protected routes: Check in route load functions or middleware

## Special Directories

**build/:**
- Purpose: Production build artifacts
- Generated: Yes (by `npm run build`)
- Committed: No
- Contents: Compiled app, client assets, server handler bundle

**.svelte-kit/:**
- Purpose: SvelteKit framework cache and generated types
- Generated: Yes (by SvelteKit during build and dev)
- Committed: No
- Contents: Type definitions, compiled files, adapter output

**node_modules/:**
- Purpose: npm package dependencies
- Generated: Yes (by `npm install` or `pnpm install`)
- Committed: No
- Managed by: pnpm-lock.yaml

**static/:**
- Purpose: Static files served as-is
- Generated: No
- Committed: Yes
- Access: `/filename` in browser (e.g., `/favicon.png`)

## File Organization Patterns

**SvelteKit Conventions in Use:**

1. **Routing**: File structure mirrors URL paths
   - `/src/routes/+page.svelte` → route `/`
   - `/src/routes/login/+page.svelte` → route `/login`
   - `/src/routes/api/login/+server.ts` → endpoint `POST /api/login`

2. **Server vs Client**: File location determines where code runs
   - `+server.ts`: Runs only on server
   - `+page.server.ts`: Loads data and handles form actions on server
   - `.svelte` files: Run on client
   - `src/lib/server/`: Server-only modules (never bundled for client)

3. **Shared Types**: Single source of truth in `src/lib/types.ts`
   - Both server and client import from same file
   - TypeScript ensures type safety across boundary

---

*Structure analysis: 2026-02-04*
