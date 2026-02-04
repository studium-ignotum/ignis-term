<script lang="ts">
	import { Xterm, XtermAddon } from '@battlefieldduck/xterm-svelte';
	import type { Terminal, ITerminalOptions, FitAddon, WebglAddon } from '@battlefieldduck/xterm-svelte';
	import { onDestroy } from 'svelte';
	import { TERMINAL_RESIZE_DEBOUNCE_MS, TERMINAL_MIN_COLS, TERMINAL_MIN_ROWS } from '../../shared/constants';
	import { terminalStore } from '$lib/stores/terminal.svelte';

	// ---------------------------------------------------------------------------
	// Props
	// ---------------------------------------------------------------------------

	let {
		sessionId,
		options = {} as ITerminalOptions,
		onInput,
		onBinaryInput,
		onTerminalResize,
	}: {
		/** Session ID this terminal displays — used for store registration */
		sessionId: string;
		/** xterm.js terminal options (theme, font, cursor, scrollback, etc.) */
		options?: ITerminalOptions;
		/** Called when user types or pastes — send as terminal_input */
		onInput?: (data: string) => void;
		/** Called for non-UTF-8 binary data (e.g. certain mouse reports) */
		onBinaryInput?: (data: string) => void;
		/** Called when terminal dimensions change — send as terminal_resize */
		onTerminalResize?: (cols: number, rows: number) => void;
	} = $props();

	// ---------------------------------------------------------------------------
	// Internal state
	// ---------------------------------------------------------------------------

	let terminal = $state<Terminal>();
	let fitAddonRef: FitAddon | null = null;
	let resizeObserver: ResizeObserver | null = null;
	let resizeTimeout: ReturnType<typeof setTimeout> | undefined;
	let webglAddonRef: WebglAddon | null = null;

	// ---------------------------------------------------------------------------
	// Addon loading (called when xterm-svelte Xterm component is ready)
	// ---------------------------------------------------------------------------

	async function handleLoad(term: Terminal): Promise<void> {
		terminal = term;

		// Register with terminal store so incoming data can be routed here
		terminalStore.registerTerminal(sessionId, term);

		// -- WebGL renderer (with DOM fallback) --------------------------------
		try {
			const { WebglAddon } = await XtermAddon.WebglAddon();
			const webgl = new WebglAddon();
			webgl.onContextLoss(() => {
				console.warn('[Terminal] WebGL context lost, falling back to DOM renderer');
				webgl.dispose();
				webglAddonRef = null;
			});
			term.loadAddon(webgl);
			webglAddonRef = webgl;
		} catch {
			console.warn('[Terminal] WebGL not available, using DOM renderer');
		}

		// -- FitAddon (responsive resize) --------------------------------------
		try {
			const { FitAddon } = await XtermAddon.FitAddon();
			const fitAddon = new FitAddon();
			term.loadAddon(fitAddon);
			fitAddonRef = fitAddon;
		} catch (e) {
			console.error('[Terminal] Failed to load FitAddon:', e);
		}

		// -- ClipboardAddon (OSC 52 clipboard) ---------------------------------
		try {
			const { ClipboardAddon } = await XtermAddon.ClipboardAddon();
			term.loadAddon(new ClipboardAddon());
		} catch (e) {
			console.warn('[Terminal] ClipboardAddon not available:', e);
		}

		// -- ImageAddon (sixel + iTerm2 inline images) -------------------------
		try {
			const { ImageAddon } = await XtermAddon.ImageAddon();
			term.loadAddon(new ImageAddon());
		} catch (e) {
			console.warn('[Terminal] ImageAddon not available:', e);
		}

		// -- WebLinksAddon (clickable URLs) ------------------------------------
		try {
			const { WebLinksAddon } = await XtermAddon.WebLinksAddon();
			term.loadAddon(new WebLinksAddon());
		} catch (e) {
			console.warn('[Terminal] WebLinksAddon not available:', e);
		}

		// -- Unicode11Addon (better unicode/emoji rendering) -------------------
		try {
			const { Unicode11Addon } = await XtermAddon.Unicode11Addon();
			term.loadAddon(new Unicode11Addon());
			term.unicode.activeVersion = '11';
		} catch (e) {
			console.warn('[Terminal] Unicode11Addon not available:', e);
		}

		// -- Set up ResizeObserver with debounced fit --------------------------
		const container = term.element?.parentElement;
		if (container && fitAddonRef) {
			resizeObserver = new ResizeObserver(() => {
				clearTimeout(resizeTimeout);
				resizeTimeout = setTimeout(() => {
					if (
						container.clientWidth > 0 &&
						container.clientHeight > 0 &&
						fitAddonRef
					) {
						fitAddonRef.fit();
					}
				}, TERMINAL_RESIZE_DEBOUNCE_MS);
			});
			resizeObserver.observe(container);

			// Initial fit after addons loaded
			requestAnimationFrame(() => {
				if (fitAddonRef && container.clientWidth > 0 && container.clientHeight > 0) {
					fitAddonRef.fit();
				}
			});
		}
	}

	// ---------------------------------------------------------------------------
	// Event handlers (passed as props to Xterm component)
	// ---------------------------------------------------------------------------

	function handleData(data: string): void {
		onInput?.(data);
	}

	function handleBinary(data: string): void {
		onBinaryInput?.(data);
	}

	function handleResize(data: { cols: number; rows: number }): void {
		// Guard against zero/tiny dimensions
		if (data.cols >= TERMINAL_MIN_COLS && data.rows >= TERMINAL_MIN_ROWS) {
			onTerminalResize?.(data.cols, data.rows);
		}
	}

	// ---------------------------------------------------------------------------
	// Public API: write data to this terminal
	// ---------------------------------------------------------------------------

	/**
	 * Write raw terminal output data to this terminal instance.
	 * Called by the terminal store when terminal_data messages arrive.
	 */
	export function write(data: string): void {
		terminal?.write(data);
	}

	/**
	 * Get the underlying Terminal instance (for store registration).
	 */
	export function getTerminal(): Terminal | undefined {
		return terminal;
	}

	/**
	 * Trigger a fit (e.g. after options change).
	 */
	export function fit(): void {
		fitAddonRef?.fit();
	}

	// ---------------------------------------------------------------------------
	// Cleanup
	// ---------------------------------------------------------------------------

	onDestroy(() => {
		clearTimeout(resizeTimeout);
		resizeObserver?.disconnect();
		resizeObserver = null;
		webglAddonRef = null;
		fitAddonRef = null;
		terminalStore.unregisterTerminal(sessionId);
		// xterm-svelte handles terminal.dispose() on its own destroy
	});
</script>

<div class="terminal-container">
	<Xterm
		bind:terminal
		{options}
		onLoad={handleLoad}
		onData={handleData}
		onBinary={handleBinary}
		onResize={handleResize}
	/>
</div>

<style>
	.terminal-container {
		width: 100%;
		height: 100%;
		overflow: hidden;
	}

	.terminal-container :global(.xterm) {
		height: 100%;
		padding: 4px;
	}
</style>
