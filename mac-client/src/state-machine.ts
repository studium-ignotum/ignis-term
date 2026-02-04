/**
 * Connection state machine for Mac client.
 * Ensures valid state transitions and prevents invalid jumps.
 */

export type ConnectionState =
  | 'disconnected'
  | 'connecting'
  | 'authenticating'  // Waiting for session code from relay
  | 'connected'       // Registered with code, ready for pairing
  | 'reconnecting';

/**
 * Valid state transitions map.
 * Each state maps to an array of states it can transition to.
 */
const STATE_TRANSITIONS: Record<ConnectionState, ConnectionState[]> = {
  disconnected: ['connecting'],
  connecting: ['authenticating', 'disconnected'],
  authenticating: ['connected', 'disconnected'],
  connected: ['reconnecting', 'disconnected'],
  reconnecting: ['connecting', 'disconnected'],
};

/**
 * Check if a state transition is valid.
 * @param from Current state
 * @param to Target state
 * @returns true if the transition is allowed
 */
export function canTransition(from: ConnectionState, to: ConnectionState): boolean {
  return STATE_TRANSITIONS[from]?.includes(to) ?? false;
}
