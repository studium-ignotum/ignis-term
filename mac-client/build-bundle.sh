#!/bin/bash
# Build Terminal Remote.app bundle
#
# This script creates a proper macOS .app bundle from the built binary.
# Run from the project root directory after building with:
#   cargo build --release --manifest-path mac-client/Cargo.toml
#
# The resulting app will:
# - Run as a menu bar-only app (no Dock icon)
# - Support macOS 13.0+

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
# Binary is in mac-client/target/release since Cargo.toml is in mac-client/
BINARY_PATH="$SCRIPT_DIR/target/release/mac-client"
APP_NAME="Terminal Remote.app"
APP_PATH="$PROJECT_ROOT/$APP_NAME"

echo "Building Terminal Remote.app..."

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Please run: cargo build --release --manifest-path mac-client/Cargo.toml"
    exit 1
fi

# Remove old bundle if it exists
if [ -d "$APP_PATH" ]; then
    echo "Removing existing bundle..."
    rm -rf "$APP_PATH"
fi

# Create bundle structure
echo "Creating bundle structure..."
mkdir -p "$APP_PATH/Contents/MacOS"
mkdir -p "$APP_PATH/Contents/Resources"

# Copy binary
echo "Copying binary..."
cp "$BINARY_PATH" "$APP_PATH/Contents/MacOS/"

# Copy Info.plist
echo "Copying Info.plist..."
cp "$SCRIPT_DIR/Info.plist" "$APP_PATH/Contents/"

# Copy icon if it exists (optional - for future .icns file)
if [ -f "$SCRIPT_DIR/resources/AppIcon.icns" ]; then
    echo "Copying icon..."
    cp "$SCRIPT_DIR/resources/AppIcon.icns" "$APP_PATH/Contents/Resources/"
fi

echo ""
echo "Bundle created: $APP_PATH"
echo ""
echo "To run: open \"$APP_PATH\""
echo "To install: cp -r \"$APP_PATH\" /Applications/"
