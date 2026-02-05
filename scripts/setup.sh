#!/bin/bash
#
# iTerm2 Remote - Setup Script
#
# This script automates the complete setup process:
# - Checks prerequisites (Node.js, pnpm, Python, iTerm2)
# - Installs all dependencies
# - Sets up Python virtual environment
# - Configures environment files
# - Optionally sets up Cloudflare Tunnel
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get script directory (project root)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║           iTerm2 Remote - Setup Script                     ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# ─────────────────────────────────────────────────────────────────
# Helper functions
# ─────────────────────────────────────────────────────────────────

print_step() {
    echo -e "\n${BLUE}▶ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

check_command() {
    if command -v "$1" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# ─────────────────────────────────────────────────────────────────
# Check prerequisites
# ─────────────────────────────────────────────────────────────────

print_step "Checking prerequisites..."

MISSING_DEPS=()

# Check Node.js
if check_command node; then
    NODE_VERSION=$(node --version | cut -d'v' -f2 | cut -d'.' -f1)
    if [ "$NODE_VERSION" -ge 20 ]; then
        print_success "Node.js $(node --version)"
    else
        print_warning "Node.js $(node --version) found, but v20+ recommended"
    fi
else
    print_error "Node.js not found"
    MISSING_DEPS+=("node")
fi

# Check pnpm
if check_command pnpm; then
    print_success "pnpm $(pnpm --version)"
else
    print_warning "pnpm not found, attempting to install..."
    if check_command npm; then
        npm install -g pnpm
        print_success "pnpm installed"
    else
        print_error "Cannot install pnpm without npm"
        MISSING_DEPS+=("pnpm")
    fi
fi

# Check Python
if check_command python3; then
    PYTHON_VERSION=$(python3 --version | cut -d' ' -f2)
    print_success "Python $PYTHON_VERSION"
else
    print_error "Python 3 not found"
    MISSING_DEPS+=("python3")
fi

# Check iTerm2
if [ -d "/Applications/iTerm.app" ]; then
    print_success "iTerm2 installed"
else
    print_error "iTerm2 not found in /Applications"
    MISSING_DEPS+=("iTerm2")
fi

# Exit if missing critical dependencies
if [ ${#MISSING_DEPS[@]} -gt 0 ]; then
    echo ""
    print_error "Missing dependencies: ${MISSING_DEPS[*]}"
    echo ""
    echo "Please install missing dependencies:"
    echo "  - Node.js: https://nodejs.org or 'brew install node'"
    echo "  - pnpm: 'npm install -g pnpm' or 'brew install pnpm'"
    echo "  - Python 3: Usually pre-installed on macOS"
    echo "  - iTerm2: https://iterm2.com or 'brew install --cask iterm2'"
    exit 1
fi

# ─────────────────────────────────────────────────────────────────
# Install Node.js dependencies
# ─────────────────────────────────────────────────────────────────

print_step "Installing Node.js dependencies..."

echo "  Installing relay-server dependencies..."
cd "$PROJECT_ROOT/relay-server"
pnpm install --silent
print_success "relay-server dependencies installed"

echo "  Installing ui dependencies..."
cd "$PROJECT_ROOT/ui"
pnpm install --silent
print_success "ui dependencies installed"

echo "  Installing mac-client dependencies..."
cd "$PROJECT_ROOT/mac-client"
pnpm install --silent
print_success "mac-client dependencies installed"

cd "$PROJECT_ROOT"

# ─────────────────────────────────────────────────────────────────
# Set up Python virtual environment
# ─────────────────────────────────────────────────────────────────

print_step "Setting up Python virtual environment..."

cd "$PROJECT_ROOT/mac-client"

if [ -d ".venv" ]; then
    print_warning "Virtual environment already exists, updating..."
else
    echo "  Creating virtual environment..."
    python3 -m venv .venv
fi

echo "  Installing iterm2 package..."
.venv/bin/pip install --quiet --upgrade pip
.venv/bin/pip install --quiet iterm2

print_success "Python environment ready"

cd "$PROJECT_ROOT"

# ─────────────────────────────────────────────────────────────────
# Configure environment files
# ─────────────────────────────────────────────────────────────────

print_step "Configuring environment files..."

# UI .env
if [ ! -f "$PROJECT_ROOT/ui/.env" ]; then
    cp "$PROJECT_ROOT/ui/.env.example" "$PROJECT_ROOT/ui/.env"
    print_success "Created ui/.env from template"
else
    print_warning "ui/.env already exists, skipping"
fi

# ─────────────────────────────────────────────────────────────────
# Make coprocess script executable
# ─────────────────────────────────────────────────────────────────

print_step "Setting up Mac client scripts..."

chmod +x "$PROJECT_ROOT/mac-client/coprocess-bridge.sh"
print_success "coprocess-bridge.sh is executable"

# ─────────────────────────────────────────────────────────────────
# Check iTerm2 Python API
# ─────────────────────────────────────────────────────────────────

print_step "Checking iTerm2 Python API..."

echo -e "${YELLOW}"
echo "┌─────────────────────────────────────────────────────────────┐"
echo "│  IMPORTANT: Enable iTerm2 Python API                        │"
echo "│                                                             │"
echo "│  1. Open iTerm2                                             │"
echo "│  2. Go to iTerm2 → Preferences (⌘,)                        │"
echo "│  3. Navigate to General → Magic                             │"
echo "│  4. Check 'Enable Python API'                               │"
echo "│                                                             │"
echo "│  The Mac client will NOT work without this setting!        │"
echo "└─────────────────────────────────────────────────────────────┘"
echo -e "${NC}"

read -p "Have you enabled the Python API in iTerm2? (y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    print_warning "Please enable the Python API before running the Mac client"
fi

# ─────────────────────────────────────────────────────────────────
# Cloudflare Tunnel setup (optional)
# ─────────────────────────────────────────────────────────────────

print_step "Cloudflare Tunnel setup (optional)..."

if check_command cloudflared; then
    print_success "cloudflared is installed"

    echo ""
    read -p "Do you want to set up a Cloudflare Tunnel for remote access? (y/n) " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""
        echo "Cloudflare Tunnel Options:"
        echo "  1. Quick tunnel (temporary, no account needed)"
        echo "  2. Persistent tunnel (requires Cloudflare account)"
        echo "  3. Skip for now"
        echo ""
        read -p "Choose option (1/2/3): " -n 1 -r TUNNEL_OPTION
        echo ""

        case $TUNNEL_OPTION in
            1)
                echo ""
                print_success "Quick tunnel selected"
                echo "Run this command when you want to start the tunnel:"
                echo ""
                echo "  cloudflared tunnel --url http://localhost:8080"
                echo ""
                echo "Then update ui/.env with the generated URL"
                ;;
            2)
                echo ""
                echo "Setting up persistent tunnel..."
                echo ""

                # Check if logged in
                if ! cloudflared tunnel list &> /dev/null; then
                    echo "You need to log in to Cloudflare first:"
                    cloudflared tunnel login
                fi

                read -p "Enter tunnel name (e.g., iterm2-remote): " TUNNEL_NAME

                if [ -n "$TUNNEL_NAME" ]; then
                    cloudflared tunnel create "$TUNNEL_NAME"
                    print_success "Tunnel '$TUNNEL_NAME' created"

                    echo ""
                    echo "Next steps:"
                    echo "  1. Add DNS route: cloudflared tunnel route dns $TUNNEL_NAME your-subdomain.yourdomain.com"
                    echo "  2. Run tunnel: cloudflared tunnel run $TUNNEL_NAME"
                    echo "  3. Update ui/.env with: VITE_RELAY_URL=wss://your-subdomain.yourdomain.com/browser"
                fi
                ;;
            *)
                print_warning "Skipping Cloudflare setup"
                ;;
        esac
    fi
else
    print_warning "cloudflared not installed"
    echo "  For remote access, install with: brew install cloudflared"
    echo "  See docs/SETUP.md for detailed instructions"
fi

# ─────────────────────────────────────────────────────────────────
# Create start script
# ─────────────────────────────────────────────────────────────────

print_step "Creating start script..."

cat > "$PROJECT_ROOT/scripts/start.sh" << 'STARTSCRIPT'
#!/bin/bash
#
# iTerm2 Remote - Start Script
#
# Usage:
#   ./scripts/start.sh              # Start locally
#   ./scripts/start.sh --tunnel     # Start with quick Cloudflare tunnel
#   ./scripts/start.sh --tunnel NAME # Start with named tunnel
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Parse arguments
USE_TUNNEL=false
TUNNEL_NAME=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --tunnel)
            USE_TUNNEL=true
            if [[ -n "$2" && ! "$2" =~ ^-- ]]; then
                TUNNEL_NAME="$2"
                shift
            fi
            shift
            ;;
        *)
            shift
            ;;
    esac
done

echo -e "${BLUE}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║           iTerm2 Remote - Starting Services                ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Stopping all services...${NC}"
    kill $(jobs -p) 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

# Start relay server
echo -e "${GREEN}▶ Starting relay server...${NC}"
cd "$PROJECT_ROOT/relay-server"
pnpm start &
RELAY_PID=$!
sleep 2

# Start Cloudflare tunnel if requested
if [ "$USE_TUNNEL" = true ]; then
    echo -e "${GREEN}▶ Starting Cloudflare tunnel...${NC}"
    if [ -n "$TUNNEL_NAME" ]; then
        cloudflared tunnel run "$TUNNEL_NAME" &
    else
        cloudflared tunnel --url http://localhost:8080 &
    fi
    TUNNEL_PID=$!
    sleep 3
fi

# Start UI dev server
echo -e "${GREEN}▶ Starting UI dev server...${NC}"
cd "$PROJECT_ROOT/ui"
pnpm run dev &
UI_PID=$!
sleep 2

# Start Mac client
echo -e "${GREEN}▶ Starting Mac client...${NC}"
cd "$PROJECT_ROOT/mac-client"
source .venv/bin/activate 2>/dev/null || true
pnpm run dev &
MAC_PID=$!

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}All services started!${NC}"
echo ""
echo "  Relay Server: http://localhost:8080"
echo "  Web UI:       http://localhost:5173"
echo ""
if [ "$USE_TUNNEL" = true ]; then
    echo -e "${YELLOW}  Check tunnel output above for your public URL${NC}"
    echo ""
fi
echo "  Watch the Mac client output for your session code"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"

# Wait for any process to exit
wait
STARTSCRIPT

chmod +x "$PROJECT_ROOT/scripts/start.sh"
print_success "Start script created at scripts/start.sh"

# ─────────────────────────────────────────────────────────────────
# Done!
# ─────────────────────────────────────────────────────────────────

echo ""
echo -e "${GREEN}"
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                    Setup Complete!                         ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

echo "Next steps:"
echo ""
echo "  1. Make sure iTerm2 is running with Python API enabled"
echo ""
echo "  2. Start the application:"
echo "     ${BLUE}./scripts/start.sh${NC}"
echo ""
echo "  3. Or start with Cloudflare tunnel for remote access:"
echo "     ${BLUE}./scripts/start.sh --tunnel${NC}"
echo ""
echo "  4. Open http://localhost:5173 in your browser"
echo ""
echo "  5. Enter the session code shown in the Mac client"
echo ""
echo "For detailed documentation, see:"
echo "  - README.md"
echo "  - docs/SETUP.md"
echo ""
