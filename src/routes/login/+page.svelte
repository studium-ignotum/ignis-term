<script lang="ts">
	import { goto } from '$app/navigation';
	import { connectionStore, connect } from '$lib/stores/connection.svelte';

	let sessionCode = $state('');
	let isSubmitting = $state(false);

	// Watch for successful connection
	$effect(() => {
		if (connectionStore.state === 'connected') {
			goto('/');
		}
	});

	// Reset submitting state on error
	$effect(() => {
		if (connectionStore.error) {
			isSubmitting = false;
		}
	});

	function handleSubmit(e: SubmitEvent) {
		e.preventDefault();

		// Normalize: uppercase, remove spaces
		const code = sessionCode.toUpperCase().replace(/\s/g, '');

		if (code.length !== 6) {
			return;
		}

		isSubmitting = true;
		connect(code);
	}
</script>

<svelte:head>
	<title>Connect - Claude Code Remote</title>
</svelte:head>

<div class="login-container">
	<div class="login-box">
		<h1>Connect to Terminal</h1>
		<p class="subtitle">Enter the session code shown on your Mac</p>

		<form onsubmit={handleSubmit}>
			<div class="input-wrapper">
				<label for="code" class="sr-only">Session Code</label>
				<input
					id="code"
					type="text"
					bind:value={sessionCode}
					placeholder="ABC123"
					maxlength="6"
					autocomplete="off"
					autocapitalize="characters"
					spellcheck="false"
					class="code-input"
					disabled={isSubmitting}
				/>
			</div>

			{#if connectionStore.error}
				<div class="error-box">
					{connectionStore.error}
				</div>
			{/if}

			<button
				type="submit"
				class="btn-primary"
				disabled={sessionCode.length !== 6 || isSubmitting}
			>
				{#if connectionStore.state === 'connecting' || connectionStore.state === 'authenticating'}
					Connecting...
				{:else}
					Connect
				{/if}
			</button>
		</form>
	</div>
</div>

<style>
	.login-container {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 20px;
		background: var(--bg-primary);
	}

	.login-box {
		background: var(--bg-secondary);
		padding: 40px;
		border-radius: 12px;
		width: 100%;
		max-width: 400px;
		text-align: center;
	}

	h1 {
		margin-bottom: 8px;
		font-size: 24px;
		color: var(--text-primary);
	}

	.subtitle {
		color: var(--text-secondary);
		margin-bottom: 32px;
	}

	form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}

	.code-input {
		width: 100%;
		padding: 16px;
		font-size: 28px;
		font-family: monospace;
		text-align: center;
		letter-spacing: 0.3em;
		text-transform: uppercase;
		background: var(--bg-primary);
		border: 2px solid var(--border-color);
		border-radius: 8px;
		color: var(--text-primary);
		transition: border-color 0.2s;
	}

	.code-input:focus {
		outline: none;
		border-color: var(--primary);
	}

	.code-input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.error-box {
		padding: 12px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.3);
		border-radius: 8px;
		color: var(--danger);
		font-size: 14px;
	}

	.btn-primary {
		width: 100%;
		padding: 14px;
		font-size: 16px;
		font-weight: 500;
		background: var(--primary);
		color: white;
		border: none;
		border-radius: 8px;
		cursor: pointer;
		transition: background 0.2s;
	}

	.btn-primary:hover:not(:disabled) {
		background: var(--primary-hover);
	}

	.btn-primary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
