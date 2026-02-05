# Web UI

The browser-based terminal interface built with React and xterm.js.

## Overview

This React application:
1. Provides a login page for entering session codes
2. Displays terminal output using xterm.js
3. Captures keyboard input and sends to the Mac
4. Shows available iTerm2 tabs and allows switching

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Browser                                                         │
│                                                                 │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ React Application                                       │    │
│  │                                                         │    │
│  │  ┌──────────┐    ┌──────────┐    ┌──────────────────┐  │    │
│  │  │  Login   │    │ Terminal │    │   Tab Sidebar    │  │    │
│  │  │  Page    │    │  (xterm) │    │                  │  │    │
│  │  └──────────┘    └──────────┘    └──────────────────┘  │    │
│  │                        │                                │    │
│  │                        ▼                                │    │
│  │              ┌─────────────────┐                        │    │
│  │              │ WebSocket       │                        │    │
│  │              │ Service         │                        │    │
│  │              └─────────────────┘                        │    │
│  │                        │                                │    │
│  └────────────────────────┼────────────────────────────────┘    │
│                           │                                     │
│                           ▼                                     │
│                    Relay Server                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Directory Structure

```
ui/
├── src/
│   ├── main.tsx              # React entry point
│   ├── App.tsx               # Root component with routing
│   ├── app.css               # Global styles
│   ├── routes/
│   │   ├── home/             # Terminal view (main page)
│   │   ├── login/            # Session code entry
│   │   └── settings/         # Settings modal
│   ├── lib/
│   │   ├── hooks/            # Custom React hooks
│   │   ├── stores/           # Context/state management
│   │   ├── services/         # WebSocket client
│   │   └── types/            # TypeScript definitions
│   └── shared/               # Types shared with relay server
├── build/                    # Production build output
├── vite.config.ts            # Build configuration
├── playwright.config.ts      # E2E test config
└── .env.example              # Environment template
```

## Running

```bash
# Development server
pnpm run dev

# Production build
pnpm run build

# Preview production build
pnpm run preview
```

## Environment Variables

Create `.env` from `.env.example`:

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_RELAY_URL` | `ws://localhost:8080/browser` | Relay WebSocket URL |

## Key Components

### Login Page (`routes/login/`)
- Session code input form
- 6-character code validation
- Error display for invalid codes

### Terminal View (`routes/home/`)
- xterm.js terminal emulator
- Receives terminal data from Mac via relay
- Sends keyboard input to Mac
- Handles resize events

### Tab Sidebar
- Displays list of open iTerm2 tabs
- Click to switch tabs
- Real-time updates when tabs change

## WebSocket Service

Handles all communication with the relay server:

- `connect(code)` - Join session with code
- `disconnect()` - Leave session
- `sendInput(data)` - Send keyboard input
- `selectTab(tabId)` - Switch iTerm2 tab

Events:
- `onTerminalData` - Terminal output received
- `onSessionList` - Tab list updated
- `onError` - Error occurred
- `onDisconnect` - Connection lost

## Technology Stack

| Technology | Purpose |
|------------|---------|
| React 19 | UI framework |
| TypeScript | Type safety |
| xterm.js 6 | Terminal emulation |
| React Router 7 | Page navigation |
| Vite 6 | Build tool |
| Playwright | E2E testing |

## Dependencies

- `react`, `react-dom` - UI framework
- `react-router-dom` - Routing
- `@xterm/xterm` - Terminal emulation
- `@xterm/addon-fit` - Terminal auto-resize
- `@xterm/addon-web-links` - Clickable URLs
