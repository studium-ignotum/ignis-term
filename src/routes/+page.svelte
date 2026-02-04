<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { connectionStore, disconnect, sendTerminalInput, sendTerminalResize } from '$lib/stores/connection';
	import { terminalStore } from '$lib/stores/terminal.svelte';
	import { tabsStore } from '$lib/stores/tabs.svelte';
	import Terminal from '$lib/components/Terminal.svelte';
	import TerminalTabs from '$lib/components/TerminalTabs.svelte';
	import MobileControlBar from '$lib/components/MobileControlBar.svelte';
	import ConnectionStatus from '$lib/components/ConnectionStatus.svelte';

	// Redirect to login if not connected on mount
	onMount(() => {
		if (browser && connectionStore.state !== 'connected') {
			goto('/login');
		}
	});

	// Watch for disconnection and redirect
	$effect(() => {
		if (browser && connectionStore.state === 'disconnected') {
			goto('/login');
		}
	});

	function handleDisconnect() {
		disconnect();
	}

	function handleInput(data: string) {
		const sid = terminalStore.activeSessionId;
		if (sid) {
			sendTerminalInput(sid, data);
		}
	}

	function handleBinaryInput(data: string) {
		const sid = terminalStore.activeSessionId;
		if (sid) {
			sendTerminalInput(sid, data);
		}
	}

	function handleResize(cols: number, rows: number) {
		const sid = terminalStore.activeSessionId;
		if (sid) {
			sendTerminalResize(sid, cols, rows);
		}
	}

	function handleMobileKey(data: string) {
		const sid = terminalStore.activeSessionId;
		if (sid) {
			sendTerminalInput(sid, data);
		}
	}

	let hasTabs = $derived(tabsStore.tabs.length > 0);
</script>

<svelte:head>
	<title>Terminal - Claude Code Remote</title>
</svelte:head>

<div class="terminal-page">
	<!-- Header bar -->
	<header class="header-bar">
		<ConnectionStatus />
		<div class="header-spacer"></div>
		<button class="btn-disconnect" onclick={handleDisconnect}>
			Disconnect
		</button>
	</header>

	<!-- Main content: sidebar + terminal area -->
	{#if connectionStore.isConnected && terminalStore.activeSessionId}
		<div class="main-layout">
			{#if hasTabs}
				<TerminalTabs />
			{/if}
			<div class="terminal-column">
				<div class="terminal-area">
					<Terminal
						options={terminalStore.options}
						onInput={handleInput}
						onBinaryInput={handleBinaryInput}
						onTerminalResize={handleResize}
					/>
				</div>
				<MobileControlBar onKey={handleMobileKey} />
			</div>
		</div>
	{:else}
		<main class="waiting-state">
			<div class="waiting-content">
				<div class="spinner"></div>
				<h2>Waiting for terminal session...</h2>
				<p class="waiting-detail">
					The Mac client will send terminal data once connected.
				</p>
				<div class="status-info">
					<span class="status-badge">{connectionStore.state}</span>
				</div>
			</div>
		</main>
	{/if}
</div>

<style>
	.terminal-page {
		height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg-primary, #1e1e1e);
		color: var(--text-primary, #d4d4d4);
	}

	/* -- Header bar ---------------------------------------------------------- */

	.header-bar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 12px;
		background: var(--bg-secondary, #1a1a1a);
		border-bottom: 1px solid var(--border, #333);
		flex-shrink: 0;
		min-height: 36px;
	}

	.header-spacer {
		flex: 1;
	}

	.btn-disconnect {
		padding: 4px 12px;
		background: transparent;
		color: var(--text-secondary, #888);
		border: 1px solid var(--border, #444);
		border-radius: 4px;
		cursor: pointer;
		font-size: 12px;
		transition: all 0.2s;
	}

	.btn-disconnect:hover {
		background: var(--danger, #dc2626);
		color: white;
		border-color: var(--danger, #dc2626);
	}

	/* -- Main layout --------------------------------------------------------- */

	.main-layout {
		flex: 1;
		display: flex;
		min-height: 0;
		overflow: hidden;
	}

	.terminal-column {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}

	.terminal-area {
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	/* -- Waiting state ------------------------------------------------------- */

	.waiting-state {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	.waiting-content {
		text-align: center;
	}

	.spinner {
		width: 32px;
		height: 32px;
		border: 3px solid var(--border, #333);
		border-top-color: var(--accent, #22c55e);
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
		margin: 0 auto 16px;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.waiting-content h2 {
		font-size: 18px;
		font-weight: 500;
		margin-bottom: 8px;
	}

	.waiting-detail {
		color: var(--text-secondary, #888);
		font-size: 14px;
		margin-bottom: 16px;
	}

	.status-info {
		display: inline-block;
	}

	.status-badge {
		display: inline-block;
		padding: 4px 12px;
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
		border-radius: 16px;
		font-size: 13px;
		font-weight: 500;
	}

	/* -- Responsive ---------------------------------------------------------- */

	@media (max-width: 767px) {
		.header-bar {
			padding: 4px 8px;
		}
	}
</style>
