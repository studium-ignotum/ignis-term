/**
 * Tab state management using Svelte 5 runes.
 *
 * Manages the list of iTerm2 tabs with bidirectional sync:
 * - Inbound: tab_list, tab_switch, tab_created, tab_closed from Mac client
 * - Outbound: tab_switch, tab_create, tab_close to Mac client via relay
 *
 * When the active tab changes, the terminal store is updated to show the
 * correct terminal session.
 */

import type { TabInfo } from '../../shared/protocol';
import { terminalStore } from './terminal.svelte';
import { sendMessage } from './connection.svelte';

// =============================================================================
// State (Svelte 5 Runes)
// =============================================================================

let tabs = $state<TabInfo[]>([]);
let activeTabId = $state<string | null>(null);

// =============================================================================
// Exported Store
// =============================================================================

export const tabsStore = {
	// -- Reactive getters -----------------------------------------------------
	get tabs() {
		return tabs;
	},
	get activeTabId() {
		return activeTabId;
	},
	get activeTab(): TabInfo | undefined {
		return tabs.find((t) => t.tabId === activeTabId);
	},

	// -- Inbound handlers (called by connection store) ------------------------

	/**
	 * Replace the full tab list (tab_list message from Mac).
	 * Sets the active tab based on isActive flag.
	 */
	setTabs(newTabs: TabInfo[]): void {
		tabs = newTabs;
		const active = newTabs.find((t) => t.isActive);
		if (active) {
			activeTabId = active.tabId;
			terminalStore.setActiveSession(active.sessionId);
		}
	},

	/**
	 * Handle a tab_switch message from Mac (iTerm2 user switched tab).
	 */
	handleTabSwitch(tabId: string): void {
		activeTabId = tabId;
		// Update isActive flags
		tabs = tabs.map((t) => ({ ...t, isActive: t.tabId === tabId }));
		const tab = tabs.find((t) => t.tabId === tabId);
		if (tab) {
			terminalStore.setActiveSession(tab.sessionId);
		}
	},

	/**
	 * Handle a tab_created message from Mac (new tab appeared in iTerm2).
	 */
	handleTabCreated(tab: TabInfo): void {
		// Don't add duplicate
		if (!tabs.find((t) => t.tabId === tab.tabId)) {
			tabs = [...tabs, tab];
		}
		// If the new tab is active, switch to it
		if (tab.isActive) {
			activeTabId = tab.tabId;
			tabs = tabs.map((t) => ({ ...t, isActive: t.tabId === tab.tabId }));
			terminalStore.setActiveSession(tab.sessionId);
		}
	},

	/**
	 * Handle a tab_closed message from Mac (tab removed in iTerm2).
	 */
	handleTabClosed(tabId: string): void {
		tabs = tabs.filter((t) => t.tabId !== tabId);
		// If the closed tab was active, switch to first remaining tab
		if (activeTabId === tabId) {
			const first = tabs[0];
			if (first) {
				activeTabId = first.tabId;
				tabs = tabs.map((t) => ({ ...t, isActive: t.tabId === first.tabId }));
				terminalStore.setActiveSession(first.sessionId);
			} else {
				activeTabId = null;
				terminalStore.setActiveSession(null);
			}
		}
	},

	// -- Outbound actions (user clicks in browser) ----------------------------

	/**
	 * Switch to a tab (sends tab_switch to Mac via relay).
	 */
	switchTab(tabId: string): void {
		activeTabId = tabId;
		tabs = tabs.map((t) => ({ ...t, isActive: t.tabId === tabId }));
		const tab = tabs.find((t) => t.tabId === tabId);
		if (tab) {
			terminalStore.setActiveSession(tab.sessionId);
		}
		sendMessage({ type: 'tab_switch', tabId });
	},

	/**
	 * Create a new tab (sends tab_create to Mac via relay).
	 */
	createTab(): void {
		sendMessage({ type: 'tab_create' });
	},

	/**
	 * Close a tab (sends tab_close to Mac via relay).
	 */
	closeTab(tabId: string): void {
		sendMessage({ type: 'tab_close', tabId });
	},

	/**
	 * Reset all tab state (called on disconnect).
	 */
	reset(): void {
		tabs = [];
		activeTabId = null;
	},
};
