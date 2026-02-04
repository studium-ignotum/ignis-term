/**
 * Terminal state management using Svelte 5 runes.
 *
 * Manages the active terminal session, xterm.js options, and a registry of
 * Terminal instances for routing incoming terminal_data messages to the correct
 * terminal. The store also handles iTerm2 config messages by converting them
 * to xterm.js options via the iterm-theme module.
 */

import type { Terminal, ITerminalOptions } from '@battlefieldduck/xterm-svelte';
import type { ConfigMessage } from '../../shared/protocol';
import { defaultTerminalOptions, configToXtermOptions } from '$lib/iterm-theme';

// =============================================================================
// State (Svelte 5 Runes)
// =============================================================================

let activeSessionId = $state<string | null>(null);
let terminalOptions = $state<ITerminalOptions>({ ...defaultTerminalOptions });

/**
 * Map of sessionId -> Terminal instance.
 * Terminal components register themselves here so the connection store can
 * route terminal_data messages to the right terminal.
 */
const terminals = new Map<string, Terminal>();

// =============================================================================
// Exported Store
// =============================================================================

export const terminalStore = {
	// -- Reactive getters -----------------------------------------------------
	get activeSessionId() {
		return activeSessionId;
	},
	get options() {
		return terminalOptions;
	},

	// -- Session management ---------------------------------------------------

	/**
	 * Set the active terminal session ID.
	 * Called when a session is established or when switching tabs.
	 */
	setActiveSession(sessionId: string | null): void {
		activeSessionId = sessionId;
	},

	// -- Config / theme -------------------------------------------------------

	/**
	 * Apply an iTerm2 config message to update terminal options.
	 * Converts the config to xterm.js ITerminalOptions format.
	 */
	applyConfig(config: ConfigMessage): void {
		terminalOptions = configToXtermOptions(config);
	},

	// -- Terminal instance registry -------------------------------------------

	/**
	 * Register a terminal instance for a session.
	 * Called by Terminal.svelte on mount/load.
	 */
	registerTerminal(sessionId: string, terminal: Terminal): void {
		terminals.set(sessionId, terminal);
	},

	/**
	 * Unregister a terminal instance (on component destroy).
	 */
	unregisterTerminal(sessionId: string): void {
		terminals.delete(sessionId);
	},

	/**
	 * Write data to the terminal for a given session.
	 * Called by the connection store when terminal_data messages arrive.
	 */
	writeData(sessionId: string, data: string): void {
		const terminal = terminals.get(sessionId);
		if (terminal) {
			terminal.write(data);
		}
	},

	/**
	 * Get a registered terminal instance by session ID.
	 */
	getTerminal(sessionId: string): Terminal | undefined {
		return terminals.get(sessionId);
	},
};
