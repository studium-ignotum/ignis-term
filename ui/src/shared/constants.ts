/**
 * Shared configuration constants for WebSocket relay
 */

// Session code configuration
export const SESSION_CODE_EXPIRY_MS = 5 * 60 * 1000; // 5 minutes
export const SESSION_CODE_LENGTH = 6;
// nolookalikes: excludes 1/l/I, 0/O/o, 2/Z, 5/S for human readability
export const SESSION_CODE_ALPHABET = '346789ABCDEFGHJKLMNPQRTUVWXY';

// Heartbeat configuration
export const HEARTBEAT_INTERVAL_MS = 30000; // 30 seconds
export const HEARTBEAT_TIMEOUT_MS = 10000; // 10 seconds to respond

// Terminal configuration
export const TERMINAL_RESIZE_DEBOUNCE_MS = 100;
export const TERMINAL_MIN_COLS = 20;
export const TERMINAL_MIN_ROWS = 5;
export const TERMINAL_DEFAULT_SCROLLBACK = 10000;

// Server configuration
export const DEFAULT_RELAY_PORT = 8080;
