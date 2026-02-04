<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { connectionStore, disconnect } from '$lib/stores/connection';

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
</script>

<svelte:head>
	<title>Terminal - Claude Code Remote</title>
</svelte:head>

<div class="terminal-page">
	<main class="main-content">
		<h1>Terminal</h1>
		<p class="placeholder">Terminal component will be added in Phase 2</p>

		<div class="status-info">
			<p>
				<strong>Status:</strong>
				<span class="status-badge">{connectionStore.state}</span>
			</p>
		</div>

		<button class="btn-disconnect" onclick={handleDisconnect}>
			Disconnect
		</button>
	</main>
</div>

<style>
	.terminal-page {
		min-height: 100vh;
		background: var(--bg-primary);
		color: var(--text-primary);
		padding: 24px;
	}

	.main-content {
		max-width: 800px;
		margin: 0 auto;
	}

	h1 {
		font-size: 24px;
		margin-bottom: 16px;
	}

	.placeholder {
		color: var(--text-secondary);
		margin-bottom: 24px;
	}

	.status-info {
		padding: 16px;
		background: var(--bg-secondary);
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

	.btn-disconnect {
		padding: 10px 20px;
		background: var(--danger);
		color: white;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		font-size: 14px;
		font-weight: 500;
		transition: background 0.2s;
	}

	.btn-disconnect:hover {
		background: #dc2626;
	}
</style>
