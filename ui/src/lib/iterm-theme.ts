/**
 * Converts iTerm2 config messages to xterm.js ITerminalOptions.
 *
 * The Mac client reads iTerm2 profile settings (font, colors, cursor) and sends
 * a ConfigMessage over the WebSocket. This module maps those values to the
 * xterm.js options format.
 */

import type { ITerminalOptions, ITheme } from '@xterm/xterm';
import type { ConfigMessage } from '../shared/protocol';
import { TERMINAL_DEFAULT_SCROLLBACK } from '../shared/constants';

// =============================================================================
// Default Terminal Options (dark theme fallback before iTerm2 config arrives)
// =============================================================================

const defaultTheme: ITheme = {
	foreground: '#d2d2d2',
	background: '#1e1e1e',
	cursor: '#aeafad',
	cursorAccent: '#1e1e1e',
	selectionBackground: '#3a3a5a',
	selectionForeground: undefined,
	black: '#1e1e1e',
	red: '#f44747',
	green: '#6a9955',
	yellow: '#d7ba7d',
	blue: '#569cd6',
	magenta: '#c586c0',
	cyan: '#4ec9b0',
	white: '#d4d4d4',
	brightBlack: '#808080',
	brightRed: '#f44747',
	brightGreen: '#6a9955',
	brightYellow: '#d7ba7d',
	brightBlue: '#569cd6',
	brightMagenta: '#c586c0',
	brightCyan: '#4ec9b0',
	brightWhite: '#e8e8e8',
};

// Detect mobile for font size adjustment
const isMobile = typeof window !== 'undefined' && window.innerWidth < 768;

export const defaultTerminalOptions: ITerminalOptions = {
	fontFamily: 'Menlo, Monaco, "Courier New", monospace',
	fontSize: isMobile ? 12 : 14,
	cursorStyle: 'block',
	cursorBlink: true,
	scrollback: TERMINAL_DEFAULT_SCROLLBACK,
	theme: defaultTheme,
	allowTransparency: false,
	macOptionIsMeta: false,
	customGlyphs: true,
	drawBoldTextInBrightColors: true,
	scrollOnUserInput: true,
	// Enable reflow for mobile - text reflows when terminal width changes
	// This is critical for mobile orientation changes
	convertEol: false,
	wordSeparator: ' ()[]{}\'"',
};

// =============================================================================
// Config Message -> xterm.js Options Converter
// =============================================================================

/**
 * Map iTerm2 cursor type names to xterm.js cursor style values.
 */
function mapCursorStyle(style: string): 'block' | 'underline' | 'bar' {
	switch (style) {
		case 'underline':
			return 'underline';
		case 'bar':
			return 'bar';
		default:
			return 'block';
	}
}

/**
 * Build an ITheme from the config message's theme record.
 * Missing keys fall back to the default dark theme.
 */
function buildTheme(themeRecord: Record<string, string>): ITheme {
	return {
		foreground: themeRecord['foreground'] ?? defaultTheme.foreground,
		background: themeRecord['background'] ?? defaultTheme.background,
		cursor: themeRecord['cursor'] ?? defaultTheme.cursor,
		cursorAccent: themeRecord['cursorAccent'] ?? defaultTheme.cursorAccent,
		selectionBackground: themeRecord['selectionBackground'] ?? defaultTheme.selectionBackground,
		selectionForeground: themeRecord['selectionForeground'] ?? defaultTheme.selectionForeground,
		black: themeRecord['black'] ?? defaultTheme.black,
		red: themeRecord['red'] ?? defaultTheme.red,
		green: themeRecord['green'] ?? defaultTheme.green,
		yellow: themeRecord['yellow'] ?? defaultTheme.yellow,
		blue: themeRecord['blue'] ?? defaultTheme.blue,
		magenta: themeRecord['magenta'] ?? defaultTheme.magenta,
		cyan: themeRecord['cyan'] ?? defaultTheme.cyan,
		white: themeRecord['white'] ?? defaultTheme.white,
		brightBlack: themeRecord['brightBlack'] ?? defaultTheme.brightBlack,
		brightRed: themeRecord['brightRed'] ?? defaultTheme.brightRed,
		brightGreen: themeRecord['brightGreen'] ?? defaultTheme.brightGreen,
		brightYellow: themeRecord['brightYellow'] ?? defaultTheme.brightYellow,
		brightBlue: themeRecord['brightBlue'] ?? defaultTheme.brightBlue,
		brightMagenta: themeRecord['brightMagenta'] ?? defaultTheme.brightMagenta,
		brightCyan: themeRecord['brightCyan'] ?? defaultTheme.brightCyan,
		brightWhite: themeRecord['brightWhite'] ?? defaultTheme.brightWhite,
	};
}

/**
 * Convert a ConfigMessage from the Mac client into xterm.js ITerminalOptions.
 *
 * The ConfigMessage contains font, fontSize, cursorStyle, cursorBlink,
 * scrollback, and a theme record mapping color names to hex values.
 */
export function configToXtermOptions(config: ConfigMessage): ITerminalOptions {
	// On mobile, use a smaller font size for better fit
	const isMobileNow = typeof window !== 'undefined' && window.innerWidth < 768;
	let fontSize = config.fontSize || defaultTerminalOptions.fontSize;
	if (isMobileNow && fontSize && fontSize > 12) {
		fontSize = 12;
	}

	return {
		...defaultTerminalOptions,
		fontFamily: config.font || defaultTerminalOptions.fontFamily,
		fontSize,
		cursorStyle: mapCursorStyle(config.cursorStyle),
		cursorBlink: config.cursorBlink,
		scrollback: config.scrollback || TERMINAL_DEFAULT_SCROLLBACK,
		theme: buildTheme(config.theme),
	};
}
