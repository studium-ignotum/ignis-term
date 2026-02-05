/**
 * Terminal component using direct @xterm/xterm integration.
 *
 * Creates a single xterm instance on mount and keeps it alive across
 * sessionId changes (tab switches). Only the registration in TerminalContext
 * is updated — the xterm instance and its scrollback buffer persist.
 */

import { useRef, useEffect } from 'react';
import { Terminal as XTerminal, type ITerminalOptions } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { TERMINAL_RESIZE_DEBOUNCE_MS, TERMINAL_MIN_COLS, TERMINAL_MIN_ROWS } from '../../shared/constants';
import { useTerminal } from '../context/TerminalContext';
import './Terminal.css';

interface TerminalProps {
  sessionId: string;
  options?: ITerminalOptions;
  onInput?: (data: string) => void;
  onBinaryInput?: (data: string) => void;
  onTerminalResize?: (cols: number, rows: number) => void;
}

export default function Terminal({
  sessionId,
  options = {},
  onInput,
  onBinaryInput,
  onTerminalResize,
}: TerminalProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<XTerminal | null>(null);
  const fitAddonRef = useRef<import('@xterm/addon-fit').FitAddon | null>(null);
  const webglAddonRef = useRef<import('@xterm/addon-webgl').WebglAddon | null>(null);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);
  const resizeTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const { registerTerminal, unregisterTerminal } = useTerminal();

  // Store callbacks and sessionId in refs to avoid re-running the main effect
  const onInputRef = useRef(onInput);
  const onBinaryInputRef = useRef(onBinaryInput);
  const onTerminalResizeRef = useRef(onTerminalResize);
  const sessionIdRef = useRef(sessionId);
  onInputRef.current = onInput;
  onBinaryInputRef.current = onBinaryInput;
  onTerminalResizeRef.current = onTerminalResize;
  sessionIdRef.current = sessionId;

  // Create terminal ONCE on mount — never recreate on sessionId change
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const term = new XTerminal({ ...options, allowProposedApi: true });
    terminalRef.current = term;
    term.open(container);

    // Register with terminal context for message routing
    registerTerminal(sessionIdRef.current, term);

    // Wire up event handlers
    const dataDisposable = term.onData((data) => onInputRef.current?.(data));
    const binaryDisposable = term.onBinary((data) => onBinaryInputRef.current?.(data));
    const resizeDisposable = term.onResize((data) => {
      if (data.cols >= TERMINAL_MIN_COLS && data.rows >= TERMINAL_MIN_ROWS) {
        onTerminalResizeRef.current?.(data.cols, data.rows);
      }
    });

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

      // FitAddon (responsive resize)
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

      // Set up ResizeObserver with debounced fit
      const fitAddon = fitAddonRef.current;
      if (container && fitAddon) {
        resizeObserverRef.current = new ResizeObserver(() => {
          clearTimeout(resizeTimeoutRef.current);
          resizeTimeoutRef.current = setTimeout(() => {
            if (
              container.clientWidth > 0 &&
              container.clientHeight > 0 &&
              fitAddonRef.current
            ) {
              fitAddonRef.current.fit();
            }
          }, TERMINAL_RESIZE_DEBOUNCE_MS);
        });
        resizeObserverRef.current.observe(container);

        // Initial fit — delay slightly to allow config and initial data to arrive first.
        // This fires onResize which sends terminal_resize to Mac.
        setTimeout(() => {
          requestAnimationFrame(() => {
            if (fitAddonRef.current && container.clientWidth > 0 && container.clientHeight > 0) {
              fitAddonRef.current.fit();
            }
          });
        }, 500);
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
      resizeDisposable.dispose();
      unregisterTerminal(sessionIdRef.current);
      term.dispose();
      terminalRef.current = null;
    };
    // Empty deps — create terminal once on mount, destroy on unmount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Update registration when sessionId changes (tab switch) without recreating terminal
  useEffect(() => {
    const term = terminalRef.current;
    if (!term) return;
    // Re-register under new sessionId
    registerTerminal(sessionId, term);
  }, [sessionId, registerTerminal]);

  // Apply option changes to existing terminal
  useEffect(() => {
    const term = terminalRef.current;
    console.log('[Terminal] options effect, term exists:', !!term, 'theme:', options.theme?.background);
    if (!term) return;

    if (options.theme) term.options.theme = options.theme;
    if (options.fontFamily !== undefined) term.options.fontFamily = options.fontFamily;
    if (options.fontSize !== undefined) term.options.fontSize = options.fontSize;
    if (options.cursorStyle !== undefined) term.options.cursorStyle = options.cursorStyle;
    if (options.cursorBlink !== undefined) term.options.cursorBlink = options.cursorBlink;
    if (options.scrollback !== undefined) term.options.scrollback = options.scrollback;

    // Force terminal to refresh with new options (if terminal has rows)
    if (term.rows > 0) {
      term.refresh(0, term.rows - 1);
    }

    console.log('[Terminal] Applied options, theme bg:', term.options.theme?.background);

    // Re-fit after option changes (font size may change dimensions)
    requestAnimationFrame(() => {
      fitAddonRef.current?.fit();
    });
  }, [options]);

  // Re-apply options after terminal fully initializes (fixes race condition on first load)
  useEffect(() => {
    const timer = setTimeout(() => {
      const term = terminalRef.current;
      if (!term) return;

      console.log('[Terminal] Delayed re-apply, theme bg:', options.theme?.background);
      if (options.theme) term.options.theme = options.theme;
      if (term.rows > 0) {
        term.refresh(0, term.rows - 1);
      }
    }, 150);
    return () => clearTimeout(timer);
  }, [options]);

  return <div ref={containerRef} className="terminal-container" />;
}
