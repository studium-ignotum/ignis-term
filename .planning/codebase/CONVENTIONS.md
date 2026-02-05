# Coding Conventions

**Analysis Date:** 2026-02-04

## Naming Patterns

**Files:**
- Route files use SvelteKit convention: `+page.svelte`, `+page.server.ts`, `+server.ts` for endpoints
- Component files: PascalCase (e.g., `Terminal.svelte`, `StatusBar.svelte`)
- Server modules: camelCase (e.g., `websocket.ts`, `sessions.ts`, `auth.ts`)
- Store files: camelCase with `store` suffix (e.g., `websocket.ts`, `notifications.ts`)
- Type definition files: lowercase (e.g., `types.ts`)

**Functions:**
- camelCase for all function declarations
- Examples: `createSession()`, `verifyPassword()`, `handleMessage()`, `scrollToTop()`
- Private methods use `private` keyword with camelCase (e.g., `handleItermConnection()`)
- Store factory functions: `createWebSocketStore()`, `createNotificationStore()`

**Variables:**
- camelCase for all local and module-level variables
- Examples: `ws`, `reconnectTimeout`, `sessionIds`, `lastNotificationContent`
- Constants: SCREAMING_SNAKE_CASE (e.g., `ITERM_TOKEN`, `ADMIN_PASSWORD`, `APPROVAL_PATTERNS`)
- Store subscriptions: prefixed with `$` (e.g., `$activeSessionId`, `$terminals`, `$activeTerminal`)

**Types:**
- PascalCase for all type/interface names
- Examples: `ScreenMessage`, `SessionStatusMessage`, `WebSocketState`, `TerminalSession`
- Discriminant fields in message types: camelCase or snake_case based on message direction
  - Browser-to-Server messages and Server-generated types use camelCase for fields: `sessionId`, `cursorX`
  - Type discriminant: `type: 'user_input'` (snake_case for message type field values)
- Class names: PascalCase (e.g., `SessionManager`, `WebSocketManager`)

## Code Style

**Formatting:**
- No linter/formatter currently configured
- 2-space indentation observed throughout
- Semicolons used in TypeScript files
- Semicolons omitted in Svelte markup

**Linting:**
- Not detected - no .eslintrc, eslint.config.js, or linting configuration found
- Recommendation: Consider adding ESLint with TypeScript support for consistency

**TypeScript Configuration:**
- Strict mode enabled (`"strict": true` in `tsconfig.json`)
- `checkJs` enabled for JavaScript files
- `esModuleInterop` enabled
- Module resolution: `"bundler"`

## Import Organization

**Order:**
1. External framework imports (`@sveltejs/kit`, `svelte`)
2. External library imports (`ws`, `bcryptjs`, etc.)
3. Internal app imports using path aliases (`$lib/*`, `$app/*`, `./$types`)

**Examples from codebase:**
```typescript
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';
import { verifyPassword, createSession } from '$lib/server/auth';
```

```typescript
import { writable, derived, get } from 'svelte/store';
import type { ServerToBrowserMessage } from '$lib/types';
```

**Path Aliases:**
- `$lib/` → `src/lib/` - main library code
- `$app/` → SvelteKit runtime APIs
- `./$types` → Generated SvelteKit type definitions for routes

## Error Handling

**Patterns:**

**API Route Error Response Pattern:**
```typescript
// Return error in JSON response with status code
export const POST: RequestHandler = async ({ request, cookies }) => {
	const { password } = await request.json();

	if (!password) {
		return json({ error: 'Password required' }, { status: 400 });
	}

	const valid = await verifyPassword(password);
	if (!valid) {
		return json({ error: 'Invalid password' }, { status: 401 });
	}

	return json({ success: true });
};
```

**Form Action Error Pattern:**
```typescript
// Use SvelteKit fail() helper for form actions
export const actions: Actions = {
	default: async ({ request, cookies }) => {
		const data = await request.formData();
		const password = data.get('password') as string;

		if (!password) {
			return fail(400, { error: 'Password required' });
		}

		const valid = await verifyPassword(password);
		if (!valid) {
			return fail(401, { error: 'Invalid password' });
		}

		throw redirect(302, '/');
	}
};
```

**Try-Catch Pattern:**
```typescript
// Used in WebSocket message handlers
ws.onmessage = (event) => {
	try {
		const msg: ServerToBrowserMessage = JSON.parse(event.data);
		handleMessage(msg);
	} catch (e) {
		console.error('[WS] Invalid message:', e);
	}
};
```

**Middleware/Guard Pattern:**
```typescript
// Check session and redirect if not authenticated
if (!verifySession(sessionToken)) {
	return new Response(null, {
		status: 302,
		headers: { Location: '/login' }
	});
}
```

## Logging

**Framework:** `console` object (no external logging library)

**Patterns:**
- Prefix log messages with `[CONTEXT]` in brackets for categorization
- Examples: `[WS]` for WebSocket logs, `[iTerm2]` would be iTerm-specific logs
- Used in: `websocket.ts` manager class

**Log Level Examples:**
```typescript
console.log('[WS] Connected');
console.log('[WS] Disconnected');
console.warn(`[WS] No iTerm2 client found for session ${sessionId}`);
console.error('[WS] Invalid message:', e);
console.error('[WS] Invalid message from iTerm2:', e);
console.error('[WS] Error:', error);
```

## Comments

**When to Comment:**
- Above non-obvious business logic
- Explaining security decisions (e.g., session expiration)
- Documenting limitations or TODOs

**Examples from codebase:**
```typescript
// Session expires after 7 days
const maxAge = 7 * 24 * 60 * 60 * 1000;
```

```typescript
// For simple setup, compare directly
// In production, you'd hash the stored password
return password === ADMIN_PASSWORD;
```

```typescript
// Subscribe to all sessions
send({ type: 'subscribe' });

// Forward to subscribed browsers
this.broadcastToBrowsers({...});

// Reconnect after 3 seconds
reconnectTimeout = setTimeout(() => {
	connect();
}, 3000);
```

**JSDoc/TSDoc:**
- Not used in current codebase
- Recommendation: Consider adding JSDoc for exported functions and public class methods

## Function Design

**Size:** Functions range from single-statement accessors to 50+ line event handlers

**Parameters:**
- Destructured parameters preferred for objects (e.g., `{ event, resolve }`, `{ request, cookies }`)
- Type annotations always used in TypeScript files
- Use `type` keyword for type imports in parameters

**Return Values:**
- Explicit return type annotations on all exported functions
- Async functions return `Promise<T>`
- RequestHandler functions return `Response` or `Promise<Response>`
- Store creation functions return object with methods and subscriptions

**Function Grouping Pattern:**
```typescript
// Related functions grouped together in modules
export function verifyItermToken(authHeader: string | undefined): boolean { }
export async function verifyPassword(password: string): Promise<boolean> { }
export function generateSessionToken(): string { }
export function createSession(): string { }
export function verifySession(token: string | undefined): boolean { }
export function deleteSession(token: string): void { }
```

## Module Design

**Exports:**
- All public functions use `export` keyword
- Private functions use `private` keyword (in classes) or not exported
- Type definitions exported as `export interface` or `export type`
- Default export used for Svelte components and store instances

**Barrel Files:**
- Not used currently - imports go directly to specific files
- Example: `import { sessionManager } from '$lib/server/sessions'` (direct import, not from index)

**Singleton Pattern:**
- Store instances exported as default: `export const wsManager = new WebSocketManager()`
- Ensures single instance across application

**Type Organization:**
```typescript
// types.ts groups related message types
export interface ScreenMessage { }
export interface SessionStatusMessage { }
export type ItermToServerMessage = ScreenMessage | SessionStatusMessage;

export interface InputMessage { }
export interface RequestScreenMessage { }
export type ServerToItermMessage = InputMessage | RequestScreenMessage;
```

## State Management (Svelte Stores)

**Store Factory Pattern:**
```typescript
function createWebSocketStore() {
	const { subscribe, set, update } = writable<WebSocketState>({
		connected: false,
		connecting: false
	});

	let ws: WebSocket | null = null;
	let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

	function connect() { }
	function disconnect() { }
	function send(msg: BrowserToServerMessage) { }

	return {
		subscribe,  // Expose for reactive subscriptions
		connect,
		disconnect,
		send
	};
}

export const wsStore = createWebSocketStore();
```

**Derived Stores:**
```typescript
const activeTerminal = derived(
	[activeSessionId, terminals],
	([$activeSessionId, $terminals]) => {
		if (!$activeSessionId) return null;
		return $terminals.get($activeSessionId) || null;
	}
);
```

## Svelte Component Patterns

**Reactive Variables (Svelte 5 Runes):**
```typescript
let { content = '', cursorX = 0, cursorY = 0 } = $props();  // Props declaration
let autoScroll = $state(true);  // State declaration
let terminal: any = null;

// Reactive effects
$effect(() => {
	if (terminal && content) {
		terminal.clear();
		terminal.write(content);
	}
});
```

**Lifecycle Hooks:**
```typescript
import { onMount, onDestroy } from 'svelte';

onMount(async () => {
	if (!browser) return;
	// Initialize
});

onDestroy(() => {
	// Cleanup
});
```

---

*Convention analysis: 2026-02-04*
