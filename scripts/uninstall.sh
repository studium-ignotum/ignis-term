#!/bin/bash
#
# Terminal Remote - Uninstaller
#
# Removes all Terminal Remote components:
#   mac-client app, relay-server, shell integration, LaunchAgent, IPC socket.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/studium-ignotum/iterm2-remote/master/scripts/uninstall.sh | bash
#   or: bash scripts/uninstall.sh
#

set -e

# ── Colors ─────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ── Constants ──────────────────────────────────────────────────
INSTALL_DIR="$HOME/.terminal-remote"
APP_DIR="$HOME/Applications"
APP_NAME="Terminal Remote.app"
RELAY_LABEL="com.terminal-remote.relay"
APP_LABEL="com.terminal-remote.app"
RELAY_PLIST="$HOME/Library/LaunchAgents/${RELAY_LABEL}.plist"
APP_PLIST="$HOME/Library/LaunchAgents/${APP_LABEL}.plist"
# Legacy label from v2.0 installer
LEGACY_PLIST="$HOME/Library/LaunchAgents/com.terminal-remote.launcher.plist"

echo -e "${BLUE}"
echo "========================================"
echo "  Terminal Remote - Uninstaller"
echo "========================================"
echo -e "${NC}"

# ── Confirmation prompt ───────────────────────────────────────
echo "This will uninstall Terminal Remote and remove all configuration."
echo ""

if [ -t 0 ]; then
    echo -en "${YELLOW}Continue? (y/n) ${NC}"
    read -r CONFIRM
else
    CONFIRM="y"
fi

if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
    echo "Cancelled."
    exit 0
fi

echo ""

# ── Stop running processes ────────────────────────────────────
echo -e "${BLUE}> Stopping running processes...${NC}"
pkill -f "relay-server" 2>/dev/null || true
pkill -f "Terminal Remote" 2>/dev/null || true
pkill -f "mac-client" 2>/dev/null || true
sleep 0.3
echo -e "${GREEN}  Processes stopped${NC}"

# ── Remove LaunchAgents ───────────────────────────────────────
echo -e "${BLUE}> Removing LaunchAgents...${NC}"
for PLIST in "$RELAY_PLIST" "$APP_PLIST" "$LEGACY_PLIST"; do
    if [ -f "$PLIST" ]; then
        launchctl unload "$PLIST" 2>/dev/null || true
        rm -f "$PLIST"
        echo -e "${GREEN}  Removed $(basename "$PLIST")${NC}"
    fi
done
echo -e "${GREEN}  LaunchAgents removed${NC}"

# ── Remove .app bundle ────────────────────────────────────────
echo -e "${BLUE}> Removing app bundle...${NC}"
if [ -d "$APP_DIR/$APP_NAME" ]; then
    rm -rf "$APP_DIR/$APP_NAME"
    echo -e "${GREEN}  Removed $APP_DIR/$APP_NAME${NC}"
else
    echo "  No app bundle found (skipped)"
fi

# ── Remove shell integration source lines ─────────────────────
echo -e "${BLUE}> Removing shell configuration...${NC}"

RC_FILES=(
    "$HOME/.zshrc"
    "$HOME/.bashrc"
    "$HOME/.bash_profile"
    "$HOME/.config/fish/config.fish"
)

for RC_FILE in "${RC_FILES[@]}"; do
    if [ -f "$RC_FILE" ] && grep -q "terminal-remote/init" "$RC_FILE"; then
        # Remove the comment line
        sed -i '' '/# Terminal Remote shell integration/d' "$RC_FILE"
        # Remove the source line (handles both quoted and unquoted paths)
        sed -i '' '/source.*terminal-remote\/init/d' "$RC_FILE"
        # Remove any leftover blank lines at end of file
        sed -i '' -e :a -e '/^\n*$/{$d;N;ba' -e '}' "$RC_FILE" 2>/dev/null || true
        echo -e "${GREEN}  Removed source line from $RC_FILE${NC}"
    fi
done

# ── Remove install directory ──────────────────────────────────
echo -e "${BLUE}> Removing install directory...${NC}"
if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
    echo -e "${GREEN}  Removed $INSTALL_DIR/${NC}"
else
    echo "  No install directory found (skipped)"
fi

# ── Remove IPC socket ─────────────────────────────────────────
echo -e "${BLUE}> Removing IPC socket...${NC}"
if [ -e /tmp/terminal-remote.sock ]; then
    rm -f /tmp/terminal-remote.sock
    echo -e "${GREEN}  Removed /tmp/terminal-remote.sock${NC}"
else
    echo "  No IPC socket found (skipped)"
fi

# ── Optional dependency removal ───────────────────────────────
echo ""

if [ -t 0 ]; then
    echo -en "${YELLOW}Remove cloudflared and tmux? (y/n) ${NC}"
    read -r DEP_CHOICE
else
    DEP_CHOICE="n"
fi

if [ "$DEP_CHOICE" = "y" ] || [ "$DEP_CHOICE" = "Y" ]; then
    echo -e "${BLUE}> Removing dependencies...${NC}"
    brew uninstall cloudflared 2>/dev/null || true
    brew uninstall tmux 2>/dev/null || true
    echo -e "${GREEN}  Removed cloudflared and tmux${NC}"
else
    echo "  Kept cloudflared and tmux"
fi

# ── Summary ───────────────────────────────────────────────────
echo ""
echo -e "${GREEN}========================================"
echo "  Terminal Remote uninstalled"
echo "========================================${NC}"
echo ""
echo "  Removed:"
echo "    - App bundle ($APP_DIR/$APP_NAME)"
echo "    - Install directory ($INSTALL_DIR/)"
echo "    - Shell integration source lines"
echo "    - LaunchAgents (relay-server + mac-client)"
echo "    - IPC socket"
echo ""
echo "  Restart your shell or open a new terminal to"
echo "  clear any remaining shell integration state."
echo ""
