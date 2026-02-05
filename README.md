# iTerm2 Remote

Access your Mac's iTerm2 terminal sessions from any browser, anywhere.

Your Mac connects to a relay server, and you can connect from your iPad, laptop, or phone to see and interact with all your open iTerm2 tabs as if you were sitting at your desk.

## Quick Start

### Automated Setup (Recommended)

```bash
# Clone the repository
git clone https://github.com/studium-ignotum/iterm2-remote.git
cd iterm2-remote

# Run setup script
./scripts/setup.sh

# Start all services
./scripts/start.sh
```

The setup script will:
- Check and install prerequisites
- Configure iTerm2 Python API
- Create Python virtual environment
- Install all dependencies
- Optionally set up Cloudflare Tunnel for remote access

**For detailed setup instructions, see [docs/SETUP.md](docs/SETUP.md).**

### Prerequisites

- **macOS** with iTerm2 installed
- **Node.js** 24.x or later
- **pnpm** package manager
- **Python** 3.7+ (for iTerm2 bridge)
- **iTerm2 Python API enabled** (see [iTerm2 Configuration](#iterm2-configuration))

### Manual Installation

```bash
# Clone the repository
git clone https://github.com/studium-ignotum/iterm2-remote.git
cd iterm2-remote

# Install all dependencies
cd relay-server && pnpm install && cd ..
cd ui && pnpm install && cd ..
cd mac-client && pnpm install && cd ..

# Set up Python environment for Mac client
cd mac-client
python3 -m venv .venv
source .venv/bin/activate
pip install iterm2
deactivate
cd ..
```

### Running Locally

Open three terminal windows:

**Terminal 1 - Start the Relay Server:**
```bash
cd relay-server
pnpm start
# Output: [Relay] Server listening on port 8080
```

**Terminal 2 - Start the Web UI:**
```bash
cd ui
pnpm run dev
# Opens at http://localhost:5173
```

**Terminal 3 - Start the Mac Client:**
```bash
cd mac-client
pnpm run dev
# Displays a 6-character session code
```

**Connect from Browser:**
1. Open http://localhost:5173
2. Enter the session code from the Mac client
3. Your iTerm2 terminals appear in the browser

## Architecture

```
┌─────────────┐     WebSocket     ┌──────────────┐     WebSocket     ┌─────────────┐
│  Mac Client │ ◄───────────────► │ Relay Server │ ◄───────────────► │  Browser UI │
│  (iTerm2)   │                   │   (Cloud)    │                   │  (React)    │
└─────────────┘                   └──────────────┘                   └─────────────┘
```

**Components:**

| Component | Directory | Purpose |
|-----------|-----------|---------|
| **Mac Client** | `mac-client/` | Runs on your Mac, bridges iTerm2 to the relay |
| **Relay Server** | `relay-server/` | Routes messages between Mac and browser clients |
| **Web UI** | `ui/` | React app with xterm.js for terminal display |

## Features

### Current (v1)

- Real-time terminal output streaming
- Full terminal emulation (colors, cursor positioning, ANSI sequences)
- Keyboard input from browser to iTerm2
- Special keys support (Ctrl+C, arrows, Tab, etc.)
- Copy/paste in browser terminal
- Terminal resize with window
- View and switch between iTerm2 tabs
- Session codes for secure pairing (6 characters, 5-minute expiry)
- Auto-reconnection with exponential backoff

### Coming Soon

- Performance optimizations (<100ms latency target)
- Bounded scrollback memory
- Rate limiting
- Graceful shutdown handling

## iTerm2 Configuration

The Mac client uses iTerm2's Python API. You must enable it:

1. Open **iTerm2**
2. Go to **iTerm2 → Preferences** (or press `⌘,`)
3. Navigate to **General → Magic**
4. Check **"Enable Python API"**

Without this setting, the Mac client cannot communicate with iTerm2.

## Remote Access with Cloudflare Tunnel

To access your terminal from anywhere (not just localhost):

```bash
# Install cloudflared
brew install cloudflared

# Start relay server
cd relay-server && pnpm start &

# Create a quick tunnel (generates random URL)
cloudflared tunnel --url http://localhost:8080
```

Use the generated URL (e.g., `https://random-words.trycloudflare.com`) in your browser.

For persistent tunnels with custom domains, see [docs/SETUP.md](docs/SETUP.md#cloudflare-tunnel-setup).

## Configuration

### Environment Variables

**Relay Server:**
```bash
RELAY_PORT=8080  # WebSocket server port (default: 8080)
```

**Mac Client:**
```bash
RELAY_URL=ws://localhost:8080/mac  # Relay server URL
```

**Web UI:**
Create a `.env` file from the example:
```bash
cp ui/.env.example ui/.env
```

Contents:
```bash
VITE_RELAY_URL=ws://localhost:8080/browser
```

## Project Structure

```
claude-code-remote/
├── mac-client/           # Mac-side application
│   ├── src/
│   │   ├── index.ts          # Entry point
│   │   ├── connection.ts     # WebSocket connection manager
│   │   ├── session-manager.ts # Routes I/O between relay and iTerm2
│   │   └── iterm-bridge.ts   # Python subprocess management
│   ├── iterm-bridge.py       # Python bridge to iTerm2
│   └── coprocess-bridge.sh   # Bash coprocess for terminal I/O
│
├── relay-server/         # Cloud relay server
│   ├── server.ts             # Main server entry point
│   ├── session-registry.ts   # Session pairing and lifecycle
│   └── shared/
│       ├── protocol.ts       # Zod message type definitions
│       └── constants.ts      # Configuration defaults
│
├── ui/                   # React web application
│   ├── src/
│   │   ├── App.tsx           # Root component
│   │   ├── routes/           # Page components
│   │   ├── lib/
│   │   │   ├── hooks/        # Custom React hooks
│   │   │   ├── stores/       # State management
│   │   │   └── services/     # WebSocket client
│   │   └── shared/           # Shared types
│   └── vite.config.ts        # Build configuration
│
└── .planning/            # Project documentation
    ├── PROJECT.md            # Project overview
    ├── REQUIREMENTS.md       # Detailed requirements
    ├── ROADMAP.md            # Development phases
    ├── STATE.md              # Current project state
    └── codebase/             # Architecture docs
        ├── ARCHITECTURE.md   # System design
        ├── STRUCTURE.md      # File organization
        └── STACK.md          # Technology stack
```

## Development

### Scripts

**Relay Server:**
```bash
pnpm start     # Run the server
```

**Mac Client:**
```bash
pnpm run dev   # Run with hot reload (tsx)
pnpm run build # Compile TypeScript
pnpm start     # Run compiled version
```

**Web UI:**
```bash
pnpm run dev      # Development server with hot reload
pnpm run build    # Production build
pnpm run preview  # Preview production build
```

### Technology Stack

| Layer | Technology |
|-------|------------|
| **Language** | TypeScript 5.0 |
| **Runtime** | Node.js 24.x |
| **WebSocket** | ws 8.18.0 |
| **Validation** | Zod 4.3.6 |
| **UI Framework** | React 19 |
| **Terminal** | xterm.js 6.0.0 |
| **Build Tool** | Vite 6.0 |
| **Package Manager** | pnpm |

### Message Protocol

All components communicate via Zod-validated WebSocket messages:

**Browser → Relay:**
- `JoinMessage` - Join session with code
- `UserInputMessage` - Send keyboard input
- `SelectTabMessage` - Switch iTerm2 tab

**Mac → Relay:**
- `SessionDataMessage` - Terminal output and tab updates

**Relay → Browser:**
- `JoinedMessage` - Session join confirmation
- `TerminalDataMessage` - Terminal output to display
- `SessionListMessage` - Available tabs

For full protocol details, see `relay-server/shared/protocol.ts`.

## How It Works

### Connection Flow

1. **Mac client** starts and connects to relay server via WebSocket
2. **Relay server** generates a 6-character session code
3. **Mac client** displays the code to the user
4. **User** enters the code in the browser
5. **Browser** connects to relay and joins the session
6. **Relay** pairs the browser with the Mac client

### Terminal I/O Flow

```
User types in browser
        │
        ▼
Browser sends UserInput message
        │
        ▼
Relay routes to Mac client
        │
        ▼
Mac client writes to iTerm2 via Python bridge
        │
        ▼
iTerm2 executes command
        │
        ▼
Terminal output captured by coprocess
        │
        ▼
Mac client sends TerminalData message
        │
        ▼
Relay broadcasts to all connected browsers
        │
        ▼
Browser renders in xterm.js
```

### Session Codes

- 6 characters using `ABCDEFGHJKMNPQRSTVWXYZ23456789` (no lookalike chars)
- Expire after 5 minutes if unused
- Never expire once a browser connects
- Case-insensitive entry

## Troubleshooting

### "Connection refused" error

1. Ensure the relay server is running (`pnpm start` in `relay-server/`)
2. Check the `RELAY_URL` matches the relay server address
3. Verify port 8080 is not blocked by firewall

### Session code not working

- Codes expire after 5 minutes - get a fresh code from the Mac client
- Codes are case-insensitive
- Ensure Mac client is still connected to relay

### Terminal not displaying output

1. Check the Mac client console for errors
2. Ensure iTerm2 is running
3. Verify the Python bridge started successfully

### Browser shows "Disconnected"

- Check relay server is still running
- The connection will auto-reconnect (1-30 second backoff)
- Refresh the page if reconnection fails

## Documentation

### Setup & Usage

| Document | Description |
|----------|-------------|
| [docs/SETUP.md](docs/SETUP.md) | **Complete setup guide** with iTerm2 config and Cloudflare Tunnel |
| [mac-client/README.md](mac-client/README.md) | Mac client architecture and data flow |
| [relay-server/README.md](relay-server/README.md) | Relay server protocol and session management |
| [ui/README.md](ui/README.md) | Web UI components and WebSocket service |

### Project Planning (in `.planning/`)

| Document | Description |
|----------|-------------|
| `PROJECT.md` | Project overview and requirements |
| `REQUIREMENTS.md` | Detailed v1 requirements with status |
| `ROADMAP.md` | Development phases and progress |
| `codebase/ARCHITECTURE.md` | System architecture deep dive |
| `codebase/STRUCTURE.md` | Where to find and add code |
| `codebase/STACK.md` | Full technology stack details |
| `codebase/CONCERNS.md` | Known issues and tech debt |

## Security Notes

This project is currently in development and should not be used in production without addressing:

- Session codes provide basic access control (not authentication)
- Terminal input is passed directly to iTerm2 (no sanitization)
- No rate limiting on connections or messages
- No audit logging

For production use, consider:
- Adding proper authentication (OAuth, JWT)
- Implementing rate limiting
- Adding input validation
- Enabling TLS for all connections
- Adding session revocation capability

## Contributing

1. Read `.planning/PROJECT.md` for project context
2. Check `.planning/codebase/STRUCTURE.md` for where to add code
3. Follow existing patterns in the codebase
4. Ensure TypeScript compiles without errors

## License

[Add license information]
