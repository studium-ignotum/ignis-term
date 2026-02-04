<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { connectionStore, disconnect, sendTerminalInput, sendTerminalResize } from '$lib/stores/connection';
	import { terminalStore } from '$lib/stores/terminal.svelte';
	import Terminal from '$lib/components/Terminal.svelte';

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
		// Binary input (e.g. certain mouse reports) sent same way as text input
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
</script>

<svelte:head>
	<title>Terminal - Claude Code Remote</title>
</svelte:head>

<div class="terminal-page">
	{#if connectionStore.isConnected && terminalStore.activeSessionId}
		<div class="terminal-area">
			<Terminal
				options={terminalStore.options}
				onInput={handleInput}
				onBinaryInput={handleBinaryInput}
				onTerminalResize={handleResize}
			/>
		</div>
	{:else}
		<main class="main-content">
			<h1>Terminal</h1>
			<p class="placeholder">Waiting for terminal session...</p>

			<div class="status-info">
				<p>
					<strong>Status:</strong>
					<span class="status-badge">{connectionStore.state}</span>
				</p>
			</div>
		</main>
	{/if}

	<div class="toolbar">
		<span class="status-dot" class:connected={connectionStore.isConnected}></span>
		<span class="status-text">{connectionStore.state}</span>
		<button class="btn-disconnect" onclick={handleDisconnect}>
			Disconnect
		</button>
	</div>
</div>

<style>
	.terminal-page {
		height: 100vh;
		display: flex;
		flex-direction: column;
		background: var(--bg-primary, #1e1e1e);
		color: var(--text-primary, #d4d4d4);
	}

	.terminal-area {
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}

	.main-content {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 24px;
	}

	h1 {
		font-size: 24px;
		margin-bottom: 16px;
	}

	.placeholder {
		color: var(--text-secondary, #888);
		margin-bottom: 24px;
	}

	.status-info {
		padding: 16px;
		background: var(--bg-secondary, #2a2a2a);
		border-radius: 8px;
		margin-bottom: 24px;
	}

	.status-badge {
		display: inline-block;
		padding: 4px 12px;
		background: rgba(34, 197, 94, 0.2);
		color: #22c55e;
		border-radius: 16px;
		font-size: 14px;
		font-weight: 500;
	}

	.toolbar {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px 12px;
		background: var(--bg-secondary, #2a2a2a);
		border-top: 1px solid var(--border, #333);
		font-size: 12px;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #888;
	}

	.status-dot.connected {
		background: #22c55e;
	}

	.status-text {
		color: var(--text-secondary, #888);
	}

	.btn-disconnect {
		margin-left: auto;
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
</style>
