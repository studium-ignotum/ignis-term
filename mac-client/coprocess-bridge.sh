#!/bin/bash
# coprocess-bridge.sh - Run as iTerm2 coprocess per session
#
# Connects terminal I/O to a Unix domain socket for the Python bridge.
# iTerm2 launches this script as a coprocess for each terminal session.
#
# How iTerm2 coprocesses work:
#   - STDIN  receives raw PTY output (byte-for-byte terminal data including
#            ANSI escape sequences) from the shell running in the session
#   - STDOUT is treated as keyboard input by iTerm2, fed back into the session
#
# This script bridges stdin/stdout to a Unix domain socket so the Python
# bridge (iterm-bridge.py) can relay terminal data to the Node.js Mac client.
#
# Usage: coprocess-bridge.sh <session_id> <socket_path>
#
# Arguments:
#   session_id   - iTerm2 session identifier (for logging/debugging)
#   socket_path  - Path to Unix domain socket created by the Python bridge

set -euo pipefail

SESSION_ID="$1"
SOCKET_PATH="$2"

if [ -z "$SESSION_ID" ] || [ -z "$SOCKET_PATH" ]; then
    echo "Usage: $0 <session_id> <socket_path>" >&2
    exit 1
fi

# Wait briefly for the Python bridge to set up the socket server.
# The bridge creates the socket before launching us, but there is a
# small race window for the listen backlog to be ready.
sleep 0.2

# Retry connection a few times in case the socket isn't ready yet
MAX_RETRIES=15
RETRY_DELAY=0.3

connect_with_retry() {
    local attempt=0
    while [ $attempt -lt $MAX_RETRIES ]; do
        if [ -S "$SOCKET_PATH" ]; then
            return 0
        fi
        attempt=$((attempt + 1))
        sleep "$RETRY_DELAY"
    done
    # Exit silently - this happens when mac-client restarts and old coprocesses
    # are still running. The Python bridge will terminate us and start fresh.
    return 1
}

# Cleanup handler for temporary files
cleanup() {
    local exit_code=$?
    if [ -n "${FIFO:-}" ]; then
        rm -f "$FIFO"
    fi
    if [ -n "${NC_PID:-}" ]; then
        kill "$NC_PID" 2>/dev/null || true
    fi
    exit $exit_code
}
trap cleanup EXIT INT TERM

# Wait for socket to be available
if ! connect_with_retry; then
    exit 1
fi

# Use socat to bidirectionally connect stdin/stdout to the Unix socket.
#   - stdin (PTY output)  -> socket (Python bridge receives terminal data)
#   - socket (Python bridge sends input) -> stdout (becomes keyboard input)
#
# socat is preferred: bidirectional, handles buffering correctly, single process.
# Falls back to nc + named pipe if socat is not available.
if command -v socat &>/dev/null; then
    exec socat - "UNIX-CONNECT:${SOCKET_PATH}"
else
    # Fallback: Use Python for robust, unbuffered bidirectional communication.
    # This avoids issues with cat buffering, nc behavior differences, and pipe management.
    # checking for python3 or python availability
    PYTHON_CMD=""
    if command -v python3 &>/dev/null; then
        PYTHON_CMD="python3"
    elif command -v python &>/dev/null; then
        PYTHON_CMD="python"
    fi

    if [ -n "$PYTHON_CMD" ]; then
        exec "$PYTHON_CMD" -u -c '
import sys
import socket
import select
import os
import signal

socket_path = sys.argv[1]

def cleanup(signum, frame):
    sys.exit(0)

signal.signal(signal.SIGTERM, cleanup)
signal.signal(signal.SIGINT, cleanup)

try:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
        s.connect(socket_path)
        
        # Set non-blocking mode for select
        s.setblocking(0)
        
        # Standard input is file descriptor 0
        # Standard output is file descriptor 1
        
        # Loop to relay data
        while True:
            # Watch for input from stdin (0) and socket (s)
            r, _, _ = select.select([0, s], [], [])
            
            for fd in r:
                if fd == 0:
                    # Stdin -> Socket
                    try:
                        data = os.read(0, 4096)
                        if not data:
                            sys.exit(0) # EOF
                        s.sendall(data)
                    except OSError:
                        sys.exit(0)
                elif fd == s:
                    # Socket -> Stdout
                    try:
                        data = s.recv(4096)
                        if not data:
                            sys.exit(0) # Remote closed
                        os.write(1, data)
                    except OSError:
                        sys.exit(0)
except Exception as e:
    # Fail silently to avoid spamming terminal output
    sys.exit(1)
' "$SOCKET_PATH"
    else
        echo "Error: socat or python required for coprocess bridge" >&2
        exit 1
    fi
fi
