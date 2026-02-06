/**
 * WebSocket message protocol definitions using Zod schemas.
 * Provides runtime validation and TypeScript types for all messages.
 */

import { z } from 'zod';

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
 * Session info in session list
 */
export const SessionInfoSchema = z.object({
  id: z.string(),
  name: z.string(),
});
export type SessionInfoSchema = z.infer<typeof SessionInfoSchema>;

/**
 * List of all active sessions.
 * Sent when a browser first connects.
 */
export const SessionListMessage = z.object({
  type: z.literal('session_list'),
  sessions: z.array(SessionInfoSchema),
});
export type SessionListMessage = z.infer<typeof SessionListMessage>;

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
