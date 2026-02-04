<script lang="ts">
	/**
	 * Floating special keys bar for mobile/touch devices.
	 *
	 * Provides buttons for keys that are hard or impossible to type on a
	 * mobile software keyboard: Esc, Tab, Ctrl+key, Alt/Meta, pipe, tilde,
	 * and arrow keys.
	 *
	 * Ctrl and Alt are "sticky" — press once to activate for the next key,
	 * press again to deactivate.
	 */

	let {
		onKey,
	}: {
		/** Called with the string to send to the terminal */
		onKey: (data: string) => void;
	} = $props();

	let ctrlActive = $state(false);
	let altActive = $state(false);

	function sendKey(key: string) {
		if (ctrlActive) {
			// Ctrl+key: send char code minus 64 (e.g. Ctrl+C = \x03)
			if (key.length === 1 && key >= 'A' && key <= 'Z') {
				onKey(String.fromCharCode(key.charCodeAt(0) - 64));
			} else if (key.length === 1 && key >= 'a' && key <= 'z') {
				onKey(String.fromCharCode(key.charCodeAt(0) - 96));
			} else {
				onKey(key);
			}
			ctrlActive = false;
		} else if (altActive) {
			// Alt sends ESC prefix (\x1b) before the key
			onKey('\x1b' + key);
			altActive = false;
		} else {
			onKey(key);
		}
	}

	function toggleCtrl() {
		ctrlActive = !ctrlActive;
		if (ctrlActive) altActive = false;
	}

	function toggleAlt() {
		altActive = !altActive;
		if (altActive) ctrlActive = false;
	}
</script>

<div class="mobile-control-bar">
	<button class="key-btn" onclick={() => sendKey('\x1b')} title="Escape">Esc</button>
	<button class="key-btn" onclick={() => sendKey('\t')} title="Tab">Tab</button>
	<button
		class="key-btn modifier"
		class:active={ctrlActive}
		onclick={toggleCtrl}
		title="Ctrl (sticky)"
	>Ctrl</button>
	<button
		class="key-btn modifier"
		class:active={altActive}
		onclick={toggleAlt}
		title="Alt (sticky)"
	>Alt</button>
	<button class="key-btn" onclick={() => sendKey('|')} title="Pipe">|</button>
	<button class="key-btn" onclick={() => sendKey('~')} title="Tilde">~</button>
	<div class="arrow-group">
		<button class="key-btn arrow" onclick={() => sendKey('\x1b[A')} title="Up" aria-label="Arrow up">&#9650;</button>
		<button class="key-btn arrow" onclick={() => sendKey('\x1b[B')} title="Down" aria-label="Arrow down">&#9660;</button>
		<button class="key-btn arrow" onclick={() => sendKey('\x1b[D')} title="Left" aria-label="Arrow left">&#9664;</button>
		<button class="key-btn arrow" onclick={() => sendKey('\x1b[C')} title="Right" aria-label="Arrow right">&#9654;</button>
	</div>
</div>

<style>
	.mobile-control-bar {
		display: none;
		align-items: center;
		gap: 4px;
		padding: 6px 8px;
		background: var(--bg-secondary, #1a1a1a);
		border-top: 1px solid var(--border, #333);
		overflow-x: auto;
		flex-shrink: 0;
		-webkit-overflow-scrolling: touch;
	}

	/* Show on narrow screens / touch devices */
	@media (max-width: 767px) {
		.mobile-control-bar {
			display: flex;
		}
	}

	/* Also show if device supports touch (wide tablets in portrait) */
	@media (pointer: coarse) {
		.mobile-control-bar {
			display: flex;
		}
	}

	.key-btn {
		flex-shrink: 0;
		padding: 6px 10px;
		min-width: 36px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-tertiary, #2a2a2a);
		color: var(--text-primary, #d4d4d4);
		border: 1px solid var(--border, #444);
		border-radius: 4px;
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
		transition: all 0.1s;
		-webkit-tap-highlight-color: transparent;
		user-select: none;
	}

	.key-btn:active {
		background: var(--bg-active, #444);
		transform: scale(0.95);
	}

	.key-btn.modifier {
		font-weight: 600;
		color: var(--text-secondary, #888);
	}

	.key-btn.modifier.active {
		background: var(--accent, #22c55e);
		color: #000;
		border-color: var(--accent, #22c55e);
	}

	.arrow-group {
		display: flex;
		gap: 2px;
		margin-left: 4px;
	}

	.key-btn.arrow {
		min-width: 30px;
		padding: 6px 6px;
		font-size: 10px;
	}
</style>
