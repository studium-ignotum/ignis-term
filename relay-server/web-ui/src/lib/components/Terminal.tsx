/**
 * Terminal component using direct @xterm/xterm integration.
 *
 * Each instance owns a single xterm terminal for its lifetime.
 * One Terminal component is created per session — show/hide is handled
 * by the parent via CSS, so the xterm buffer is never cleared on tab switch.
 *
 * Terminal dimensions come from the mac (via session_resize messages).
 * On mobile (<768px), mac size is ignored and FitAddon fits to screen.
 */

import { useRef, useEffect } from 'react';
import { Terminal as XTerminal, type ITerminalOptions } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { TERMINAL_MIN_COLS, TERMINAL_MIN_ROWS } from '../../shared/constants';
import { useTerminal } from '../context/TerminalContext';
import './Terminal.css';

interface TerminalProps {
  sessionId: string;
  options?: ITerminalOptions;
  onInput?: (data: string) => void;
  onBinaryInput?: (data: string) => void;
}

export default function Terminal({
  sessionId,
  options = {},
  onInput,
  onBinaryInput,
}: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTerminal | null>(null);
  const fitAddonRef = useRef<import('@xterm/addon-fit').FitAddon | null>(null);
  const webglAddonRef = useRef<import('@xterm/addon-webgl').WebglAddon | null>(null);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const resizeTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  /** Mac's terminal dimensions (source of truth on desktop). Null until first session_resize. */
  const macSizeRef = useRef<{ cols: number; rows: number } | null>(null);

  const { registerTerminal, unregisterTerminal, markTerminalReady, onSessionResize } = useTerminal();

  // Store callbacks in refs to avoid re-running the main effect
  const onInputRef = useRef(onInput);
  const onBinaryInputRef = useRef(onBinaryInput);
  onInputRef.current = onInput;
  onBinaryInputRef.current = onBinaryInput;

  // Create terminal ONCE on mount — never recreate on sessionId change
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const term = new XTerminal({ ...options, allowProposedApi: true });
    terminalRef.current = term;
    term.open(container);

    // Register with terminal context for message routing
    registerTerminal(sessionId, term);

    // Wire up event handlers
    const dataDisposable = term.onData((data) => onInputRef.current?.(data));
    const binaryDisposable = term.onBinary((data) => onBinaryInputRef.current?.(data));

    /**
     * Fit terminal to container.
     * Mobile (<768px): always use FitAddon (ignore mac size, fit to screen).
     * Desktop: use mac's exact cols/rows if known, otherwise FitAddon.
     */
    const fitTerminal = () => {
      if (!fitAddonRef.current) return;
      if (container.clientWidth <= 0 || container.clientHeight <= 0) return;

      if (window.innerWidth < 768) {
        // Mobile: fit to screen, ignore mac dimensions
        fitAddonRef.current.fit();
        return;
      }

      if (macSizeRef.current) {
        const { cols, rows } = macSizeRef.current;
        term.resize(cols, rows);
      } else {
        fitAddonRef.current.fit();
      }
    };

    /**
     * Handle resize from mac: store dimensions and fit.
     */
    const handleMacResize = (cols: number, rows: number) => {
      if (cols < TERMINAL_MIN_COLS || rows < TERMINAL_MIN_ROWS) return;
      macSizeRef.current = { cols, rows };
      fitTerminal();
    };

    // Subscribe to mac resize events for this session
    const unsubResize = onSessionResize(sessionId, handleMacResize);

    // Load addons asynchronously
    (async () => {
      // WebGL renderer (with DOM fallback)
      try {
        const { WebglAddon } = await import('@xterm/addon-webgl');
        const webgl = new WebglAddon();
        webgl.onContextLoss(() => {
          console.warn('[Terminal] WebGL context lost, falling back to DOM renderer');
          webgl.dispose();
          webglAddonRef.current = null;
        });
        term.loadAddon(webgl);
        webglAddonRef.current = webgl;
      } catch {
        console.warn('[Terminal] WebGL not available, using DOM renderer');
      }

      // FitAddon
      try {
        const { FitAddon } = await import('@xterm/addon-fit');
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        fitAddonRef.current = fitAddon;
      } catch (e) {
        console.error('[Terminal] Failed to load FitAddon:', e);
      }

      // ClipboardAddon (OSC 52 clipboard)
      try {
        const { ClipboardAddon } = await import('@xterm/addon-clipboard');
        term.loadAddon(new ClipboardAddon());
      } catch (e) {
        console.warn('[Terminal] ClipboardAddon not available:', e);
      }

      // ImageAddon (sixel + iTerm2 inline images)
      try {
        const { ImageAddon } = await import('@xterm/addon-image');
        term.loadAddon(new ImageAddon());
      } catch (e) {
        console.warn('[Terminal] ImageAddon not available:', e);
      }

      // WebLinksAddon (clickable URLs)
      try {
        const { WebLinksAddon } = await import('@xterm/addon-web-links');
        term.loadAddon(new WebLinksAddon());
      } catch (e) {
        console.warn('[Terminal] WebLinksAddon not available:', e);
      }

      // Unicode11Addon (better unicode/emoji rendering)
      try {
        const { Unicode11Addon } = await import('@xterm/addon-unicode11');
        term.loadAddon(new Unicode11Addon());
        term.unicode.activeVersion = '11';
      } catch (e) {
        console.warn('[Terminal] Unicode11Addon not available:', e);
      }

      // Set up ResizeObserver — re-fit when container changes
      if (container) {
        resizeObserverRef.current = new ResizeObserver(() => {
          clearTimeout(resizeTimeoutRef.current);
          resizeTimeoutRef.current = setTimeout(() => {
            if (container.clientWidth > 0 && container.clientHeight > 0) {
              fitTerminal();
            }
          }, 100);
        });
        resizeObserverRef.current.observe(container);

        // Initial fit (retry a few times to handle async layout)
        const doInitialFit = () => {
          if (fitAddonRef.current && container.clientWidth > 0 && container.clientHeight > 0) {
            fitTerminal();
            term.refresh(0, term.rows - 1);
            markTerminalReady(sessionId);
            term.scrollToBottom();
            term.focus();
          }
        };
        requestAnimationFrame(doInitialFit);
        setTimeout(doInitialFit, 50);
        setTimeout(doInitialFit, 150);
        setTimeout(doInitialFit, 300);
        setTimeout(doInitialFit, 600);
        setTimeout(doInitialFit, 1000);
      }
    })();

    // Cleanup — only on unmount
    return () => {
      clearTimeout(resizeTimeoutRef.current);
      resizeObserverRef.current?.disconnect();
      resizeObserverRef.current = null;
      webglAddonRef.current = null;
      fitAddonRef.current = null;
      dataDisposable.dispose();
      binaryDisposable.dispose();
      unsubResize();
      unregisterTerminal(sessionId);
      term.dispose();
      terminalRef.current = null;
    };
    // Empty deps — create terminal once on mount, destroy on unmount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Apply option changes to existing terminal
  useEffect(() => {
    const term = terminalRef.current;
    if (!term) return;

    if (options.theme) term.options.theme = options.theme;
    if (options.fontFamily !== undefined) term.options.fontFamily = options.fontFamily;
    if (options.fontSize !== undefined) term.options.fontSize = options.fontSize;
    if (options.cursorStyle !== undefined) term.options.cursorStyle = options.cursorStyle;
    if (options.cursorBlink !== undefined) term.options.cursorBlink = options.cursorBlink;
    if (options.scrollback !== undefined) term.options.scrollback = options.scrollback;

    if (term.rows > 0) {
      term.refresh(0, term.rows - 1);
    }

    requestAnimationFrame(() => {
      fitAddonRef.current?.fit();
    });
  }, [options]);

  // Re-apply options after terminal fully initializes (fixes race condition on first load)
  useEffect(() => {
    const timer = setTimeout(() => {
      const term = terminalRef.current;
      if (!term) return;
      if (options.theme) term.options.theme = options.theme;
      if (term.rows > 0) {
        term.refresh(0, term.rows - 1);
      }
    }, 150);
    return () => clearTimeout(timer);
  }, [options]);

  return <div ref={containerRef} className="terminal-container" />;
}
