# Complete Setup Guide

This guide walks you through setting up iTerm2 Remote from scratch, including iTerm2 configuration and optional Cloudflare Tunnel for remote access.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Setup (Automated)](#quick-setup-automated)
3. [Manual Setup](#manual-setup)
4. [iTerm2 Configuration](#iterm2-configuration)
5. [Cloudflare Tunnel Setup](#cloudflare-tunnel-setup)
6. [Running the Application](#running-the-application)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Required Software

| Software | Version | Installation |
|----------|---------|--------------|
| **macOS** | 10.15+ | - |
| **iTerm2** | 3.4+ | [Download](https://iterm2.com/downloads.html) or `brew install --cask iterm2` |
| **Node.js** | 24.x+ | [Download](https://nodejs.org) or `brew install node` |
| **pnpm** | 9.x+ | `npm install -g pnpm` or `brew install pnpm` |
| **Python** | 3.7+ | Usually pre-installed on macOS |

### Optional (for remote access)

| Software | Purpose | Installation |
|----------|---------|--------------|
| **cloudflared** | Expose relay server to internet | `brew install cloudflared` |

---

## Quick Setup (Automated)

Run the setup script to install everything automatically:

```bash
# From the project root
./scripts/setup.sh
```

The script will:
1. Check and install prerequisites
2. Configure iTerm2 Python API
3. Create Python virtual environment
4. Install all dependencies
5. Set up environment files
6. Optionally configure Cloudflare Tunnel

After setup, start the application:

```bash
./scripts/start.sh
```

---

## Manual Setup

### Step 1: Clone and Install Dependencies

```bash
# Clone the repository
git clone https://github.com/studium-ignotum/iterm2-remote.git
cd iterm2-remote

# Install relay server dependencies
cd relay-server
pnpm install

# Install UI dependencies
cd ../ui
pnpm install

# Install Mac client dependencies
cd ../mac-client
pnpm install
cd ..
```

### Step 2: Set Up Python Environment

The Mac client uses Python to communicate with iTerm2. Create a virtual environment:

```bash
cd mac-client

# Create virtual environment
python3 -m venv .venv

# Activate it
source .venv/bin/activate

# Install iTerm2 Python package
pip install iterm2

# Deactivate when done
deactivate

cd ..
```

### Step 3: Configure Environment Variables

```bash
# Create UI environment file
cp ui/.env.example ui/.env

# Edit if needed (default works for local development)
# VITE_RELAY_URL=ws://localhost:8080/browser
```

---

## iTerm2 Configuration

### Enable Python API

The Mac client uses iTerm2's Python API to access terminal sessions. You must enable it:

1. Open **iTerm2**
2. Go to **iTerm2 → Preferences** (or press `⌘,`)
3. Navigate to **General → Magic**
4. Check **"Enable Python API"**

![iTerm2 Python API Setting](https://iterm2.com/python-api/tutorial/images/magic.png)

### Verify Python API is Working

Test that the Python API is accessible:

```bash
cd mac-client
source .venv/bin/activate
python3 -c "import iterm2; print('iTerm2 Python API is available')"
deactivate
```

If you see an error, ensure:
- iTerm2 is running
- Python API is enabled (see above)
- You're using the correct Python (the one with `iterm2` installed)

### Grant Accessibility Permissions (if prompted)

When first running, macOS may prompt for accessibility permissions:

1. Go to **System Preferences → Security & Privacy → Privacy → Accessibility**
2. Add and enable **iTerm2**
3. You may also need to add **Terminal** if running from Terminal

---

## Cloudflare Tunnel Setup

Cloudflare Tunnel lets you securely expose your local relay server to the internet without opening firewall ports.

### Install cloudflared

```bash
# macOS
brew install cloudflared

# Or download from:
# https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/
```

### Option A: Quick Tunnel (No Account Required)

For quick testing, use a temporary tunnel:

```bash
# Start the relay server first
cd relay-server && pnpm start &

# Create a quick tunnel (generates random URL)
cloudflared tunnel --url http://localhost:8080
```

You'll see output like:
```
Your quick Tunnel has been created! Visit it at:
https://random-words-here.trycloudflare.com
```

Use this URL in your browser (replace `ws://localhost:8080` with `wss://random-words-here.trycloudflare.com`).

### Option B: Persistent Tunnel (Recommended)

For a permanent setup with a custom domain:

#### 1. Login to Cloudflare

```bash
cloudflared tunnel login
```

This opens a browser to authenticate with your Cloudflare account.

#### 2. Create a Tunnel

```bash
# Create tunnel (choose a name)
cloudflared tunnel create iterm2-remote

# Note the tunnel ID (e.g., a1b2c3d4-e5f6-...)
```

#### 3. Configure DNS

```bash
# Route your subdomain to the tunnel
cloudflared tunnel route dns iterm2-remote terminal.yourdomain.com
```

#### 4. Create Configuration File

Create `~/.cloudflared/config.yml`:

```yaml
tunnel: <your-tunnel-id>
credentials-file: /Users/<username>/.cloudflared/<tunnel-id>.json

ingress:
  # WebSocket endpoint for relay server
  - hostname: terminal.yourdomain.com
    service: http://localhost:8080
    originRequest:
      noTLSVerify: true
  # Catch-all (required)
  - service: http_status:404
```

#### 5. Run the Tunnel

```bash
# Run manually
cloudflared tunnel run iterm2-remote

# Or install as service (runs on boot)
sudo cloudflared service install
sudo launchctl start com.cloudflare.cloudflared
```

### Update UI Environment

Update `ui/.env` to use your Cloudflare URL:

```bash
# For quick tunnel
VITE_RELAY_URL=wss://random-words-here.trycloudflare.com/browser

# For persistent tunnel
VITE_RELAY_URL=wss://terminal.yourdomain.com/browser
```

---

## Running the Application

### Local Development

Open three terminal windows:

**Terminal 1 - Relay Server:**
```bash
cd relay-server
pnpm start
```

**Terminal 2 - Web UI:**
```bash
cd ui
pnpm run dev
```

**Terminal 3 - Mac Client:**
```bash
cd mac-client
pnpm run dev
```

### With Cloudflare Tunnel

**Terminal 1 - Relay Server:**
```bash
cd relay-server
pnpm start
```

**Terminal 2 - Cloudflare Tunnel:**
```bash
cloudflared tunnel --url http://localhost:8080
# Or for persistent: cloudflared tunnel run iterm2-remote
```

**Terminal 3 - Mac Client:**
```bash
cd mac-client
pnpm run dev
```

**Terminal 4 - Web UI (optional for local testing):**
```bash
cd ui
pnpm run dev
```

### Using the Start Script

After running `./scripts/setup.sh`, use the convenient start script:

```bash
# Start all services locally
./scripts/start.sh

# Start with Cloudflare quick tunnel
./scripts/start.sh --tunnel

# Start with persistent Cloudflare tunnel
./scripts/start.sh --tunnel iterm2-remote
```

---

## Troubleshooting

### "Failed to connect to iTerm2"

**Cause:** iTerm2 Python API not enabled or iTerm2 not running.

**Fix:**
1. Ensure iTerm2 is running
2. Go to **iTerm2 → Preferences → General → Magic**
3. Enable **"Enable Python API"**
4. Restart iTerm2

### "No module named 'iterm2'"

**Cause:** Python iterm2 package not installed in the correct environment.

**Fix:**
```bash
cd mac-client
source .venv/bin/activate
pip install iterm2
deactivate
```

### "Connection refused" on relay server

**Cause:** Relay server not running or wrong port.

**Fix:**
1. Ensure relay server is running: `cd relay-server && pnpm start`
2. Check port 8080 is not in use: `lsof -i :8080`
3. Verify `RELAY_URL` environment variable matches

### Session code not working

**Cause:** Code expired or Mac client disconnected.

**Fix:**
1. Session codes expire after 5 minutes if unused
2. Check Mac client is connected to relay
3. Get a fresh code from Mac client output

### Cloudflare tunnel not working

**Cause:** Various configuration issues.

**Fix:**
1. Ensure relay server is running locally first
2. For quick tunnels, check the URL in cloudflared output
3. For persistent tunnels, verify DNS is propagated: `dig terminal.yourdomain.com`
4. Check tunnel status: `cloudflared tunnel info iterm2-remote`

### Terminal shows blank or no output

**Cause:** Coprocess not attached or iTerm2 session issue.

**Fix:**
1. Check Mac client logs for errors
2. Ensure iTerm2 has at least one open terminal
3. Try creating a new tab in iTerm2
4. Restart the Mac client

### WebSocket connection drops frequently

**Cause:** Network instability or timeout settings.

**Fix:**
1. Check your network connection
2. For Cloudflare tunnels, ensure WebSocket support is enabled
3. The client auto-reconnects with exponential backoff (1-30 seconds)

---

## Security Considerations

### Local Development
- Session codes provide basic access control
- All communication is unencrypted (localhost only)

### Production/Remote Access
- **Always use Cloudflare Tunnel** or similar secure tunnel
- Tunnel provides TLS encryption automatically
- Never expose port 8080 directly to the internet
- Consider adding authentication layer for production use

### Best Practices
1. Use persistent Cloudflare tunnels with your own domain
2. Restrict tunnel access with Cloudflare Access policies
3. Regularly rotate session codes
4. Monitor tunnel logs for suspicious activity
