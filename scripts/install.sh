#!/bin/bash
#
# Terminal Remote - Installer
#
# Single-command installation of the complete Terminal Remote stack:
#   mac-client menu bar app, relay-server, dependencies, and shell integration.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/studium-ignotum/iterm2-remote/master/scripts/install.sh | bash
#
# Or with a specific version:
#   VERSION=v2.0.0 curl -fsSL ... | bash
#
# Re-running updates binaries without duplicating shell config lines.
#

set -e

# ── Colors ─────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ── Constants ──────────────────────────────────────────────────
REPO="studium-ignotum/iterm2-remote"
VERSION="${VERSION:-latest}"
INSTALL_DIR="$HOME/.terminal-remote"
APP_DIR="$HOME/Applications"
APP_NAME="Terminal Remote.app"
RELAY_LABEL="com.terminal-remote.relay"
APP_LABEL="com.terminal-remote.app"
RELAY_PLIST="$HOME/Library/LaunchAgents/${RELAY_LABEL}.plist"
APP_PLIST="$HOME/Library/LaunchAgents/${APP_LABEL}.plist"

# ── Temp directory with cleanup trap ───────────────────────────
TMPDIR=""
cleanup() {
    if [ -n "$TMPDIR" ] && [ -d "$TMPDIR" ]; then
        rm -rf "$TMPDIR"
    fi
}
trap cleanup EXIT

echo -e "${BLUE}"
echo "========================================"
echo "  Terminal Remote - Installer"
echo "========================================"
echo -e "${NC}"

# ── Architecture detection ─────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
    arm64|x86_64)
        echo -e "${GREEN}  Architecture: $ARCH${NC}"
        ;;
    *)
        echo -e "${RED}Error: Unsupported architecture '$ARCH'.${NC}"
        echo "  Terminal Remote supports arm64 (Apple Silicon) and x86_64 (Intel)."
        exit 1
        ;;
esac

# ── Homebrew check ─────────────────────────────────────────────
echo ""
echo -e "${BLUE}> Checking prerequisites...${NC}"

if ! command -v brew &>/dev/null; then
    echo -e "${RED}Error: Homebrew is required to install dependencies (cloudflared, tmux).${NC}"
    echo ""
    echo "  Install Homebrew first:"
    echo "    /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
    echo ""
    echo "  Or visit: https://brew.sh"
    exit 1
fi
echo -e "${GREEN}  Homebrew found${NC}"

# ── Dependency installation ────────────────────────────────────
for dep in cloudflared tmux; do
    if command -v "$dep" &>/dev/null; then
        echo -e "${GREEN}  $dep already installed${NC}"
    else
        echo -e "${BLUE}  Installing $dep...${NC}"
        if ! brew install "$dep"; then
            echo -e "${RED}Error: Failed to install $dep via Homebrew.${NC}"
            exit 1
        fi
        echo -e "${GREEN}  $dep installed${NC}"
    fi
done

# ── Resolve download URL ──────────────────────────────────────
echo ""
echo -e "${BLUE}> Downloading Terminal Remote...${NC}"

if [ "$VERSION" = "latest" ]; then
    VERSION=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo -e "${RED}Error: Could not resolve latest release version.${NC}"
        echo "  Check https://github.com/$REPO/releases for available releases."
        exit 1
    fi
    echo -e "  Latest version: ${GREEN}$VERSION${NC}"
fi

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/terminal-remote-$VERSION-darwin-$ARCH.tar.gz"
echo "  URL: $DOWNLOAD_URL"

TMPDIR=$(mktemp -d)

if ! curl -fsSL -o "$TMPDIR/terminal-remote.tar.gz" "$DOWNLOAD_URL"; then
    echo -e "${RED}Error: Download failed.${NC}"
    echo "  Release '$VERSION' may not exist for architecture '$ARCH'."
    echo "  Check https://github.com/$REPO/releases for available releases."
    exit 1
fi

echo "  Extracting..."
tar -xzf "$TMPDIR/terminal-remote.tar.gz" -C "$TMPDIR"

# ── Install binaries ──────────────────────────────────────────
echo ""
echo -e "${BLUE}> Installing binaries...${NC}"

mkdir -p "$INSTALL_DIR/bin"
mkdir -p "$APP_DIR"

# Install relay-server binary
cp "$TMPDIR/relay-server" "$INSTALL_DIR/bin/relay-server"
chmod +x "$INSTALL_DIR/bin/relay-server"
echo -e "${GREEN}  relay-server -> $INSTALL_DIR/bin/relay-server${NC}"

# Install .app bundle (remove old version first for clean update)
rm -rf "$APP_DIR/$APP_NAME"
cp -R "$TMPDIR/$APP_NAME" "$APP_DIR/"
echo -e "${GREEN}  $APP_NAME -> $APP_DIR/$APP_NAME${NC}"

# ── Install shell integration ─────────────────────────────────
echo ""
echo -e "${BLUE}> Installing shell integration...${NC}"

cp "$TMPDIR/shell-integration/init.zsh" "$INSTALL_DIR/"
cp "$TMPDIR/shell-integration/init.bash" "$INSTALL_DIR/"
cp "$TMPDIR/shell-integration/init.fish" "$INSTALL_DIR/"
echo -e "${GREEN}  Shell init scripts installed to $INSTALL_DIR/${NC}"

# ── Configure shell ───────────────────────────────────────────
echo ""
echo -e "${BLUE}> Configuring shell...${NC}"

CURRENT_SHELL=$(basename "$SHELL")

case "$CURRENT_SHELL" in
    zsh)
        RC_FILE="$HOME/.zshrc"
        SHELL_EXT="zsh"
        ;;
    bash)
        if [ -f "$HOME/.bashrc" ]; then
            RC_FILE="$HOME/.bashrc"
        elif [ -f "$HOME/.bash_profile" ]; then
            RC_FILE="$HOME/.bash_profile"
        else
            RC_FILE="$HOME/.bashrc"
        fi
        SHELL_EXT="bash"
        ;;
    fish)
        RC_FILE="$HOME/.config/fish/config.fish"
        SHELL_EXT="fish"
        mkdir -p "$HOME/.config/fish"
        ;;
    *)
        echo -e "${YELLOW}  Unknown shell '$CURRENT_SHELL'. Skipping shell configuration.${NC}"
        echo "  Add this to your shell config manually:"
        echo "    source \"$INSTALL_DIR/init.zsh\"  (for zsh)"
        RC_FILE=""
        SHELL_EXT=""
        ;;
esac

SOURCE_LINE="source \"$INSTALL_DIR/init.$SHELL_EXT\""

if [ -n "$RC_FILE" ]; then
    # Create rc file if it doesn't exist
    touch "$RC_FILE"

    if grep -qF "terminal-remote/init." "$RC_FILE"; then
        echo -e "${GREEN}  Shell integration already configured in $RC_FILE${NC}"
    else
        echo "" >> "$RC_FILE"
        echo "# Terminal Remote shell integration" >> "$RC_FILE"
        echo "$SOURCE_LINE" >> "$RC_FILE"
        echo -e "${GREEN}  Added to $RC_FILE:${NC}"
        echo "    $SOURCE_LINE"
    fi
fi

# ── Create LaunchAgents ───────────────────────────────────────
echo ""
echo -e "${BLUE}> Setting up LaunchAgents...${NC}"

mkdir -p "$HOME/Library/LaunchAgents"

# Remove legacy v2.0 LaunchAgent if present
LEGACY_PLIST="$HOME/Library/LaunchAgents/com.terminal-remote.launcher.plist"
if [ -f "$LEGACY_PLIST" ]; then
    launchctl unload "$LEGACY_PLIST" 2>/dev/null || true
    rm -f "$LEGACY_PLIST"
    # Also remove the old launcher script
    rm -f "$INSTALL_DIR/bin/terminal-remote-start"
    echo -e "${GREEN}  Cleaned up legacy LaunchAgent${NC}"
fi

# Unload current agents if already installed (for clean reinstall)
launchctl unload "$RELAY_PLIST" 2>/dev/null || true
launchctl unload "$APP_PLIST" 2>/dev/null || true

# relay-server daemon
cat > "$RELAY_PLIST" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${RELAY_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/bin/relay-server</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>${INSTALL_DIR}/relay-server.log</string>
    <key>StandardErrorPath</key>
    <string>${INSTALL_DIR}/relay-server.log</string>
</dict>
</plist>
EOF

echo -e "${GREEN}  Created $RELAY_PLIST${NC}"

# mac-client menu bar app
cat > "$APP_PLIST" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${APP_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${APP_DIR}/Terminal Remote.app/Contents/MacOS/mac-client</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>${INSTALL_DIR}/mac-client.log</string>
    <key>StandardErrorPath</key>
    <string>${INSTALL_DIR}/mac-client.log</string>
</dict>
</plist>
EOF

echo -e "${GREEN}  Created $APP_PLIST${NC}"

# Start both services immediately
launchctl load "$RELAY_PLIST"
launchctl load "$APP_PLIST"
echo -e "${GREEN}  Services started${NC}"

# ── Summary ───────────────────────────────────────────────────
echo ""
echo -e "${GREEN}========================================"
echo "  Installation complete!"
echo "========================================${NC}"
echo ""
echo "  Installed:"
echo "    App:          $APP_DIR/$APP_NAME"
echo "    Relay:        $INSTALL_DIR/bin/relay-server"
echo "    Shell config: $INSTALL_DIR/init.$SHELL_EXT"
echo ""
echo "  Services are running and will auto-start on login."
echo "  Restart your shell to activate shell integration,"
echo "  then new terminal sessions will auto-register."
echo ""
echo "  Uninstall:"
echo -e "    ${BLUE}curl -fsSL https://raw.githubusercontent.com/$REPO/master/scripts/uninstall.sh | bash${NC}"
echo ""
