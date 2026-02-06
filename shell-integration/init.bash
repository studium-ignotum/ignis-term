# Terminal Remote - Bash Integration (PTY Proxy)
# Source this file in .bashrc: source ~/.terminal-remote/init.bash
#
# Wraps the shell in pty-proxy for transparent terminal capture.
# Unlike tmux, this does NOT affect scroll, copy, mouse, or any
# terminal behavior — the terminal emulator works 100% natively.

# Skip if already inside pty-proxy (prevent recursion)
# PTY_PROXY_ACTIVE can leak to new Terminal windows via macOS env inheritance,
# so verify by checking if our parent is actually pty-proxy.
if [[ -n "$PTY_PROXY_ACTIVE" ]]; then
  case "$(ps -o comm= -p $PPID 2>/dev/null)" in
    login|*/login) unset PTY_PROXY_ACTIVE ;;  # New Terminal window — leaked env var
    *) return ;;  # Inside pty-proxy or subshell — legitimate
  esac
fi

# Skip if not interactive
[[ $- != *i* ]] && return

# Skip if running inside tmux, screen, or other multiplexer
[[ -n "$TMUX" || -n "$STY" ]] && return

# Skip if no TTY (e.g., scp, rsync, pipe)
[[ ! -t 0 ]] && return

_terminal_remote_find_proxy() {
  local proxy
  for proxy in \
    "$HOME/.terminal-remote/bin/pty-proxy" \
    "/usr/local/bin/pty-proxy" \
    "/opt/homebrew/bin/pty-proxy"; do
    [[ -x "$proxy" ]] && echo "$proxy" && return 0
  done
  return 1
}

_terminal_remote_init() {
  local proxy
  proxy=$(_terminal_remote_find_proxy) || return 0  # silently skip if not found

  # Check if mac-client is running (socket exists)
  [[ -S /tmp/terminal-remote.sock ]] || return 0  # silently skip

  exec "$proxy"
}

_terminal_remote_init

unset -f _terminal_remote_find_proxy 2>/dev/null
unset -f _terminal_remote_init 2>/dev/null
