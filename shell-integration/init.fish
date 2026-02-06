# Terminal Remote - Fish Integration (PTY Proxy)
# Source this file in config.fish: source ~/.terminal-remote/init.fish
#
# Wraps the shell in pty-proxy for transparent terminal capture.
# Unlike tmux, this does NOT affect scroll, copy, mouse, or any
# terminal behavior — the terminal emulator works 100% natively.

# Skip if already inside pty-proxy (prevent recursion)
if set -q PTY_PROXY_ACTIVE
    exit 0
end

# Skip if not interactive
if not status is-interactive
    exit 0
end

# Skip if running inside tmux, screen, or other multiplexer
if set -q TMUX; or set -q STY
    exit 0
end

# Skip if no TTY
if not isatty stdin
    exit 0
end

function _terminal_remote_find_proxy
    for proxy in \
        "$HOME/.terminal-remote/bin/pty-proxy" \
        "/usr/local/bin/pty-proxy" \
        "/opt/homebrew/bin/pty-proxy"
        if test -x "$proxy"
            echo "$proxy"
            return 0
        end
    end
    return 1
end

function _terminal_remote_init
    set -l proxy (_terminal_remote_find_proxy)
    or return 0  # silently skip if not found

    # Check if mac-client is running (socket exists)
    if not test -S /tmp/terminal-remote.sock
        return 0  # silently skip
    end

    exec $proxy
end

_terminal_remote_init

functions -e _terminal_remote_find_proxy 2>/dev/null
functions -e _terminal_remote_init 2>/dev/null
