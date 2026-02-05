# Testing Patterns

**Analysis Date:** 2026-02-04

## Test Framework

**Status:** No test framework currently configured

**Recommendation:** Implement testing framework for quality focus areas:
- For unit/integration: Consider Vitest (integrates well with Vite/SvelteKit)
- For E2E: Consider Playwright (recommended for SvelteKit applications)
- For component testing: Consider Vitest + svelte/testing

**Note:** Package.json does not include test script or testing dependencies.

## Test File Organization

**Current State:** No test files found in codebase

**Recommended Pattern:**

**Location:** Co-located test files
```
src/
├── lib/
│   ├── server/
│   │   ├── auth.ts
│   │   ├── auth.test.ts          # Co-located test
│   │   ├── sessions.ts
│   │   ├── sessions.test.ts
│   │   ├── websocket.ts
│   │   └── websocket.test.ts
│   ├── stores/
│   │   ├── websocket.ts
│   │   ├── websocket.test.ts
│   │   ├── notifications.ts
│   │   └── notifications.test.ts
│   └── components/
│       ├── Terminal.svelte
│       └── Terminal.test.ts
└── routes/
    ├── api/
    │   ├── login/
    │   │   ├── +server.ts
    │   │   └── +server.test.ts
    │   └── logout/
    │       ├── +server.ts
    │       └── +server.test.ts
```

**File Naming:**
- `.test.ts` or `.test.svelte` suffix
- Same basename as source file

## Priority Testing Areas

**High Priority (Critical Business Logic):**

1. **Authentication** (`src/lib/server/auth.ts`)
   - Session token generation and validation
   - Password verification
   - Session expiration (7-day TTL)
   - Functions: `verifyItermToken()`, `verifyPassword()`, `generateSessionToken()`, `verifySession()`

2. **WebSocket Communication** (`src/lib/server/websocket.ts`)
   - Message routing between iTerm2, browsers, and sessions
   - Client connection/disconnection handling
   - Message type discrimination
   - Subscription filtering
   - Functions: `handleItermMessage()`, `handleBrowserMessage()`, `broadcastToBrowsers()`, `sendToIterm()`

3. **Session Management** (`src/lib/server/sessions.ts`)
   - Session creation and updates
   - Connection state tracking
   - Session retrieval and removal
   - Class: `SessionManager`

**Medium Priority:**

4. **State Store Updates** (`src/lib/stores/websocket.ts`)
   - Message handling in store
   - Reactive updates to sessions and terminals
   - Auto-selection logic
   - Function: `handleMessage()`

5. **API Endpoints** (`src/routes/api/login/+server.ts`, `src/routes/api/logout/+server.ts`)
   - Login validation and session cookie setting
   - Logout session deletion

**Lower Priority:**

6. **Notification Detection** (`src/lib/stores/notifications.ts`)
   - Pattern matching for approval prompts
   - Notification triggering
   - Sound generation

7. **Component Rendering** (`src/lib/components/*.svelte`)
   - Terminal display updates
   - Session list rendering
   - Input handling

## Suggested Test Structure

**Unit Test Pattern for Server Functions:**

```typescript
// src/lib/server/auth.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { generateSessionToken, verifySession, verifyPassword, verifyItermToken } from './auth';

describe('auth module', () => {
	describe('generateSessionToken', () => {
		it('should generate a 64-character hex string', () => {
			const token = generateSessionToken();
			expect(token).toMatch(/^[0-9a-f]{64}$/);
		});

		it('should generate different tokens on each call', () => {
			const token1 = generateSessionToken();
			const token2 = generateSessionToken();
			expect(token1).not.toBe(token2);
		});
	});

	describe('verifyPassword', () => {
		it('should return true for valid password', async () => {
			// Set ADMIN_PASSWORD env var before test
			const result = await verifyPassword('admin');
			expect(result).toBe(true);
		});

		it('should return false for invalid password', async () => {
			const result = await verifyPassword('wrong');
			expect(result).toBe(false);
		});
	});

	describe('session lifecycle', () => {
		it('should verify a newly created session', () => {
			const token = createSession();
			expect(verifySession(token)).toBe(true);
		});

		it('should not verify a deleted session', () => {
			const token = createSession();
			deleteSession(token);
			expect(verifySession(token)).toBe(false);
		});

		it('should expire sessions after 7 days', () => {
			// Requires mocking Date.now() or injecting time
		});
	});
});
```

**WebSocket Message Handler Test Pattern:**

```typescript
// src/lib/server/websocket.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { ItermToServerMessage, ServerToBrowserMessage } from '$lib/types';
import { WebSocketManager } from './websocket';

describe('WebSocketManager', () => {
	let manager: WebSocketManager;
	let mockWss: any;

	beforeEach(() => {
		manager = new WebSocketManager();
		mockWss = {
			handleUpgrade: vi.fn(),
			on: vi.fn()
		};
	});

	describe('message routing', () => {
		it('should forward screen messages from iTerm to browsers', () => {
			const msg: ItermToServerMessage = {
				type: 'screen',
				sessionId: 'session1',
				sessionName: 'Terminal 1',
				content: 'Test output',
				cursorX: 0,
				cursorY: 5,
				timestamp: Date.now()
			};

			// Test routing logic
		});

		it('should handle session status changes', () => {
			const msg: ItermToServerMessage = {
				type: 'session_status',
				sessionId: 'session1',
				sessionName: 'Terminal 1',
				status: 'connected'
			};

			// Test status update
		});
	});
});
```

**Store Test Pattern:**

```typescript
// src/lib/stores/websocket.test.ts
import { describe, it, expect } from 'vitest';
import { get } from 'svelte/store';
import { wsStore } from './websocket';

describe('websocket store', () => {
	it('should initialize with disconnected state', () => {
		const state = get(wsStore);
		expect(state.connected).toBe(false);
		expect(state.connecting).toBe(false);
	});

	it('should update active session when message received', () => {
		// Simulate incoming message
		wsStore.selectSession('session1');
		expect(get(wsStore.activeSessionId)).toBe('session1');
	});
});
```

## Setup Requirements

**Install Testing Dependencies:**

```bash
npm install -D vitest @vitest/ui @sveltejs/adapter-node
npm install -D @testing-library/svelte  # For component testing
```

**Create vitest.config.ts:**

```typescript
import { getViteConfig } from 'astro/config';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	test: {
		globals: true,
		environment: 'jsdom',
		coverage: {
			provider: 'v8',
			reporter: ['text', 'json', 'html'],
			exclude: [
				'node_modules/',
				'build/',
				'.svelte-kit/'
			]
		}
	}
});
```

**Add Test Scripts to package.json:**

```json
{
	"scripts": {
		"test": "vitest",
		"test:ui": "vitest --ui",
		"test:coverage": "vitest --coverage"
	}
}
```

## Environment Configuration for Testing

**Required Environment Variables for Tests:**

```
ITERM_TOKEN=test-token
ADMIN_PASSWORD=test-password
```

**Mock/Test Values:**
- Session token pattern: 64-character hex string (output of `generateSessionToken()`)
- Session expiration: 7 days (604800000 ms)
- Test iTerm token: Any string matching env var

## Mocking Strategy

**What to Mock:**

1. **WebSocket connections** - Use `vi.fn()` or mock WebSocket class
2. **Date.now()** - For session expiration testing: `vi.useFakeTimers()`
3. **Environment variables** - Import from `$env/dynamic/private`
4. **File storage** - Mock localStorage in browser context tests

**What NOT to Mock:**

1. **Core business logic** - Test actual session validation and token generation
2. **Message type validation** - Test that message handlers properly discriminate types
3. **Store state transitions** - Test actual store updates

**Example Mock Pattern:**

```typescript
import { vi } from 'vitest';

// Mock WebSocket for connection tests
global.WebSocket = vi.fn(() => ({
	readyState: WebSocket.OPEN,
	send: vi.fn(),
	close: vi.fn(),
	addEventListener: vi.fn(),
	removeEventListener: vi.fn()
})) as any;

// Mock environment variables
vi.stubEnv('ITERM_TOKEN', 'test-token');
vi.stubEnv('ADMIN_PASSWORD', 'test-password');
```

## Error Scenarios to Test

**Authentication:**
- Missing password
- Invalid password
- Expired session token
- Malformed session token

**WebSocket:**
- Invalid JSON messages
- Missing message type
- Unknown message type
- Connection drop during message send
- Unauthorized iTerm connection (invalid token)
- Unauthorized browser connection (invalid session)

**Session Management:**
- Update nonexistent session
- Set connection on session that doesn't exist yet
- Retrieve nonexistent session
- Remove nonexistent session

## Coverage Goals

**Target:** 80%+ coverage for critical modules

**Critical Modules (must prioritize):**
- `src/lib/server/auth.ts` - Target 100%
- `src/lib/server/sessions.ts` - Target 95%+
- `src/lib/server/websocket.ts` - Target 85%+ (complex message routing)

**Important Modules (target 70%+):**
- `src/lib/stores/websocket.ts` - Business logic in store updates
- `src/routes/api/login/+server.ts` - Endpoint logic

**View Coverage:**

```bash
npm run test:coverage
# HTML report generated in coverage/ directory
```

---

*Testing analysis: 2026-02-04*

**Next Steps:**
1. Install Vitest and testing dependencies
2. Create `vitest.config.ts` configuration
3. Implement priority 1 tests for auth module
4. Implement priority 2 tests for WebSocket and sessions
5. Set up coverage reporting and CI integration
