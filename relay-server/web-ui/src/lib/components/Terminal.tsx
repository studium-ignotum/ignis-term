/**
 * Terminal component using direct @xterm/xterm integration.
 *
 * Each instance owns a single xterm terminal for its lifetime.
 * One Terminal component is created per session — show/hide is handled
 * by the parent via CSS, so the xterm buffer is never cleared on tab switch.
 *
 * Terminal dimensions come from the mac (via session_resize messages).
 * On mobile/small screens, the terminal is CSS-scaled to fit the container
 * rather than sending resize back to the mac.
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
  const fitReadyRef = useRef(false);
  /** Mac's terminal dimensions (source of truth). Null until first session_resize. */
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
     * Apply CSS transform to scale the terminal to fit the container.
     * Called when the mac sends a resize or the container changes size.
     */
    const applyScale = () => {
      const xtermEl = container.querySelector('.xterm') as HTMLElement | null;
      if (!xtermEl) return;

      const cw = container.clientWidth;
      const ch = container.clientHeight;
      const tw = xtermEl.scrollWidth;
      const th = xtermEl.scrollHeight;

      if (tw <= 0 || th <= 0 || cw <= 0 || ch <= 0) return;

      if (tw <= cw && th <= ch) {
        // Terminal fits — no scaling needed
        xtermEl.style.transform = '';
        xtermEl.style.transformOrigin = '';
        container.style.overflow = '';
      } else {
        // Terminal larger than container — scale to fit
        const scale = Math.min(cw / tw, ch / th);
        xtermEl.style.transformOrigin = 'top left';
        xtermEl.style.transform = `scale(${scale})`;
        container.style.overflow = 'hidden';
      }
    };

    /**
     * Handle resize from mac: set xterm to mac's exact dimensions,
     * then apply CSS scaling to fit container.
     */
    const handleMacResize = (cols: number, rows: number) => {
      macSizeRef.current = { cols, rows };
      if (cols >= TERMINAL_MIN_COLS && rows >= TERMINAL_MIN_ROWS) {
        term.resize(cols, rows);
        // Allow xterm to render at new size, then scale
        requestAnimationFrame(applyScale);
      }
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

      // FitAddon — used as fallback when no mac size is known yet
      try {
        const { FitAddon } = await import('@xterm/addon-fit');
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        fitAddonRef.current = fitAddon;
        fitReadyRef.current = true;
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

      // Set up ResizeObserver — re-apply scaling when container changes
      if (container) {
        resizeObserverRef.current = new ResizeObserver(() => {
          clearTimeout(resizeTimeoutRef.current);
          resizeTimeoutRef.current = setTimeout(() => {
            if (container.clientWidth > 0 && container.clientHeight > 0) {
              if (macSizeRef.current) {
                // Mac size known — re-apply scale
                applyScale();
              } else if (fitAddonRef.current) {
                // No mac size yet — use FitAddon as fallback
                fitAddonRef.current.fit();
              }
            }
          }, 100);
        });
        resizeObserverRef.current.observe(container);

        // Initial fit — use FitAddon until mac sends its size
        const doInitialFit = (label: string) => {
          const w = container.clientWidth;
          const h = container.clientHeight;
          if (fitAddonRef.current && w > 0 && h > 0 && !macSizeRef.current) {
            fitAddonRef.current.fit();
            term.refresh(0, term.rows - 1);
            markTerminalReady(sessionId);
            term.scrollToBottom();
            term.focus();
          } else if (macSizeRef.current) {
            markTerminalReady(sessionId);
            applyScale();
            term.scrollToBottom();
            term.focus();
          }
        };
        requestAnimationFrame(() => doInitialFit('raf'));
        setTimeout(() => doInitialFit('50ms'), 50);
        setTimeout(() => doInitialFit('150ms'), 150);
        setTimeout(() => doInitialFit('300ms'), 300);
        setTimeout(() => doInitialFit('600ms'), 600);
        setTimeout(() => doInitialFit('1000ms'), 1000);
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

    // Force terminal to refresh with new options (if terminal has rows)
    if (term.rows > 0) {
      term.refresh(0, term.rows - 1);
    }

    // Re-fit after option changes (font size may change dimensions)
    requestAnimationFrame(() => {
      if (macSizeRef.current) {
        const container = containerRef.current;
        if (container) {
          const xtermEl = container.querySelector('.xterm') as HTMLElement | null;
          if (xtermEl) {
            // Reset scale before measuring
            xtermEl.style.transform = '';
            requestAnimationFrame(() => {
              const cw = container.clientWidth;
              const ch = container.clientHeight;
              const tw = xtermEl.scrollWidth;
              const th = xtermEl.scrollHeight;
              if (tw > cw || th > ch) {
                const scale = Math.min(cw / tw, ch / th);
                xtermEl.style.transformOrigin = 'top left';
                xtermEl.style.transform = `scale(${scale})`;
                container.style.overflow = 'hidden';
              }
            });
          }
        }
      } else {
        fitAddonRef.current?.fit();
      }
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
