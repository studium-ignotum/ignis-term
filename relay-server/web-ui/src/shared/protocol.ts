/**
 * WebSocket message protocol definitions using Zod schemas.
 * Provides runtime validation and TypeScript types for all messages.
 */

import { z } from 'zod';

// =============================================================================
// Mac Client -> Relay Messages
// =============================================================================

/**
 * Mac client registers with the relay to get a session code
 */
export const RegisterMessage = z.object({
  type: z.literal('register'),
  clientId: z.string().uuid(),
});
export type RegisterMessage = z.infer<typeof RegisterMessage>;

// =============================================================================
// Relay -> Mac Client Messages
// =============================================================================

/**
 * Relay confirms registration and provides session code
 */
export const RegisteredMessage = z.object({
  type: z.literal('registered'),
  code: z.string().length(6),
  expiresAt: z.number(), // Unix timestamp in ms
});
export type RegisteredMessage = z.infer<typeof RegisteredMessage>;

// =============================================================================
// Browser -> Relay Messages
// =============================================================================

/**
 * Browser joins a session using the code
 */
export const JoinMessage = z.object({
  type: z.literal('join'),
  code: z.string().length(6),
});
export type JoinMessage = z.infer<typeof JoinMessage>;

/**
 * Browser rejoins an existing session using the sessionId (after page refresh)
 */
export const RejoinMessage = z.object({
  type: z.literal('rejoin'),
  sessionId: z.string().uuid(),
});
export type RejoinMessage = z.infer<typeof RejoinMessage>;

// =============================================================================
// Relay -> Browser Messages
// =============================================================================

/**
 * Relay confirms browser has joined the session
 */
export const JoinedMessage = z.object({
  type: z.literal('joined'),
  sessionId: z.string().uuid(),
});
export type JoinedMessage = z.infer<typeof JoinedMessage>;

// =============================================================================
// Auth Protocol Messages (Rust Relay v2)
// =============================================================================

/**
 * Browser authenticates with the relay using a session code.
 * This is the first message sent after WebSocket connection.
 * Uses snake_case to match Rust relay's serde(rename_all = "snake_case").
 */
export const AuthMessage = z.object({
  type: z.literal('auth'),
  session_code: z.string().length(6),
});
export type AuthMessage = z.infer<typeof AuthMessage>;

/**
 * Relay confirms successful authentication
 */
export const AuthSuccessMessage = z.object({
  type: z.literal('auth_success'),
});
export type AuthSuccessMessage = z.infer<typeof AuthSuccessMessage>;

/**
 * Relay rejects authentication with a reason
 */
export const AuthFailedMessage = z.object({
  type: z.literal('auth_failed'),
  reason: z.string(),
});
export type AuthFailedMessage = z.infer<typeof AuthFailedMessage>;

// =============================================================================
// Session Event Messages (Mac Client -> Browser via Relay)
// =============================================================================

/**
 * A shell session connected from the mac-client.
 * Sent when a terminal tab/window connects via IPC.
 */
export const SessionConnectedMessage = z.object({
  type: z.literal('session_connected'),
  session_id: z.string(),
  name: z.string(),
});
export type SessionConnectedMessage = z.infer<typeof SessionConnectedMessage>;

/**
 * A shell session disconnected from the mac-client.
 * Sent when a terminal tab/window closes.
 */
export const SessionDisconnectedMessage = z.object({
  type: z.literal('session_disconnected'),
  session_id: z.string(),
});
export type SessionDisconnectedMessage = z.infer<typeof SessionDisconnectedMessage>;

// =============================================================================
// Error Messages (Relay -> Any Client)
// =============================================================================

export const ErrorCode = z.enum([
  'INVALID_CODE',
  'EXPIRED_CODE',
  'ALREADY_JOINED',
  'NOT_FOUND',
  'MAC_DISCONNECTED',
  'INVALID_MESSAGE',
  'SESSION_NOT_FOUND',
]);
export type ErrorCode = z.infer<typeof ErrorCode>;

export const ErrorMessage = z.object({
  type: z.literal('error'),
  code: ErrorCode,
  message: z.string(),
});
export type ErrorMessage = z.infer<typeof ErrorMessage>;

// =============================================================================
// Bidirectional Data Messages
// =============================================================================

/**
 * Terminal data forwarded between Mac and Browser
 */
export const DataMessage = z.object({
  type: z.literal('data'),
  payload: z.string(),
});
export type DataMessage = z.infer<typeof DataMessage>;

// =============================================================================
// Health Check Messages
// =============================================================================

export const PingMessage = z.object({
  type: z.literal('ping'),
  ts: z.number(),
});
export type PingMessage = z.infer<typeof PingMessage>;

export const PongMessage = z.object({
  type: z.literal('pong'),
  ts: z.number(),
});
export type PongMessage = z.infer<typeof PongMessage>;

// =============================================================================
// Terminal I/O Messages
// =============================================================================

/**
 * Terminal output data from Mac to Browser (raw escape sequences)
 */
export const TerminalDataMessage = z.object({
  type: z.literal('terminal_data'),
  sessionId: z.string(),
  payload: z.string(),
});
export type TerminalDataMessage = z.infer<typeof TerminalDataMessage>;

/**
 * Initial terminal data from Mac to Browser (buffered content on connect/tab switch)
 */
export const InitialTerminalDataMessage = z.object({
  type: z.literal('initial_terminal_data'),
  sessionId: z.string(),
  payload: z.string(),
});
export type InitialTerminalDataMessage = z.infer<typeof InitialTerminalDataMessage>;

/**
 * Terminal input from Browser to Mac (user keystrokes)
 */
export const TerminalInputMessage = z.object({
  type: z.literal('terminal_input'),
  sessionId: z.string(),
  payload: z.string(),
});
export type TerminalInputMessage = z.infer<typeof TerminalInputMessage>;

/**
 * Terminal resize from Browser to Mac (dimensions changed)
 */
export const TerminalResizeMessage = z.object({
  type: z.literal('terminal_resize'),
  sessionId: z.string(),
  cols: z.number().int().positive(),
  rows: z.number().int().positive(),
});
export type TerminalResizeMessage = z.infer<typeof TerminalResizeMessage>;

// =============================================================================
// Tab Management Messages
// =============================================================================

/**
 * Tab metadata (reusable schema)
 */
export const TabInfo = z.object({
  tabId: z.string(),
  sessionId: z.string(),
  title: z.string(),
  isActive: z.boolean(),
});
export type TabInfo = z.infer<typeof TabInfo>;

/**
 * Full tab list update from Mac to Browser
 */
export const TabListMessage = z.object({
  type: z.literal('tab_list'),
  tabs: z.array(TabInfo),
});
export type TabListMessage = z.infer<typeof TabListMessage>;

/**
 * Request to switch active tab (bidirectional)
 */
export const TabSwitchMessage = z.object({
  type: z.literal('tab_switch'),
  tabId: z.string(),
  sessionId: z.string().optional(),  // Active session within tab (if provided by Mac)
});
export type TabSwitchMessage = z.infer<typeof TabSwitchMessage>;

/**
 * Request to create a new tab (Browser to Mac)
 */
export const TabCreateMessage = z.object({
  type: z.literal('tab_create'),
});
export type TabCreateMessage = z.infer<typeof TabCreateMessage>;

/**
 * Request to close a tab (Browser to Mac)
 */
export const TabCloseMessage = z.object({
  type: z.literal('tab_close'),
  tabId: z.string(),
});
export type TabCloseMessage = z.infer<typeof TabCloseMessage>;

/**
 * Notification that a new tab was created (Mac to Browser)
 */
export const TabCreatedMessage = z.object({
  type: z.literal('tab_created'),
  tab: TabInfo,
});
export type TabCreatedMessage = z.infer<typeof TabCreatedMessage>;

/**
 * Notification that a tab was closed (Mac to Browser)
 */
export const TabClosedMessage = z.object({
  type: z.literal('tab_closed'),
  tabId: z.string(),
});
export type TabClosedMessage = z.infer<typeof TabClosedMessage>;

// =============================================================================
// Configuration Messages
// =============================================================================

/**
 * iTerm2 configuration sent from Mac to Browser for xterm.js setup
 */
export const ConfigMessage = z.object({
  type: z.literal('config'),
  font: z.string(),
  fontSize: z.number(),
  cursorStyle: z.enum(['block', 'underline', 'bar']),
  cursorBlink: z.boolean(),
  scrollback: z.number(),
  theme: z.record(z.string(), z.string()),
});
export type ConfigMessage = z.infer<typeof ConfigMessage>;

// =============================================================================
// Discriminated Union for Incoming Messages
// =============================================================================

/**
 * All possible incoming messages that the relay can receive
 */
export const IncomingMessage = z.discriminatedUnion('type', [
  // Mac client registration
  RegisterMessage,
  // Browser auth (v2 Rust relay)
  AuthMessage,
  // Browser join (v1 legacy, kept for transition)
  JoinMessage,
  RejoinMessage,
  // Data and health
  DataMessage,
  PingMessage,
  // Terminal I/O
  TerminalDataMessage,
  InitialTerminalDataMessage,
  TerminalInputMessage,
  TerminalResizeMessage,
  // Tab management
  TabListMessage,
  TabSwitchMessage,
  TabCreateMessage,
  TabCloseMessage,
  TabCreatedMessage,
  TabClosedMessage,
  // Configuration
  ConfigMessage,
]);
export type IncomingMessage = z.infer<typeof IncomingMessage>;

/**
 * All possible outgoing messages from the relay
 */
export const OutgoingMessage = z.discriminatedUnion('type', [
  // Mac client registration response
  RegisteredMessage,
  // Browser auth responses (v2 Rust relay)
  AuthSuccessMessage,
  AuthFailedMessage,
  // Browser join response (v1 legacy, kept for transition)
  JoinedMessage,
  // Error
  ErrorMessage,
  // Data and health
  DataMessage,
  PongMessage,
  // Session events (from mac-client via relay)
  SessionConnectedMessage,
  SessionDisconnectedMessage,
  // Terminal I/O
  TerminalDataMessage,
  InitialTerminalDataMessage,
  TerminalInputMessage,
  TerminalResizeMessage,
  // Tab management
  TabListMessage,
  TabSwitchMessage,
  TabCreateMessage,
  TabCloseMessage,
  TabCreatedMessage,
  TabClosedMessage,
  // Configuration
  ConfigMessage,
]);
export type OutgoingMessage = z.infer<typeof OutgoingMessage>;

// =============================================================================
// Parsing Utilities
// =============================================================================

export type ParseResult<T> =
  | { success: true; data: T }
  | { success: false; error: string };

/**
 * Safely parse an incoming message with full validation.
 * Returns typed result indicating success or failure.
 */
export function parseMessage(raw: string): ParseResult<IncomingMessage> {
  try {
    const data = JSON.parse(raw);
    const result = IncomingMessage.safeParse(data);
    if (result.success) {
      return { success: true, data: result.data };
    }
    return { success: false, error: result.error.message };
  } catch (e) {
    return { success: false, error: 'Invalid JSON' };
  }
}

/**
 * Type-safe message creation helper
 */
export function createMessage<T extends OutgoingMessage['type']>(
  type: T,
  data: Omit<Extract<OutgoingMessage, { type: T }>, 'type'>
): string {
  return JSON.stringify({ type, ...data });
}
