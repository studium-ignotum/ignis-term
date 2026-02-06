# Terminal Remote - Fish Integration
# Connects shell sessions to mac-client for remote access
# Source this file in config.fish: source ~/.terminal-remote/init.fish

# Configuration
set -g _TERMINAL_REMOTE_SOCKET "/tmp/terminal-remote.sock"
set -g _TERMINAL_REMOTE_CONNECTED 0
set -g _TERMINAL_REMOTE_PID %self
set -g _TERMINAL_REMOTE_BG_PID ""
set -g _TERMINAL_REMOTE_WATCHER_PID ""
set -g _TERMINAL_REMOTE_WARNED 0

# Generate session name: "dirname [PID]"
function _terminal_remote_session_name
  echo (basename $PWD)" [$_TERMINAL_REMOTE_PID]"
end

# Escape string for JSON (backslash, quote, newline, tab)
function _terminal_remote_json_escape
  string replace -a '\\' '\\\\' -- $argv[1] | \
  string replace -a '"' '\\"' | \
  string replace -a \n '\\n' | \
  string replace -a \t '\\t'
end

# Attempt to connect to Terminal Remote
function _terminal_remote_connect
  set -l name (_terminal_remote_json_escape (_terminal_remote_session_name))
  set -l msg "{\"name\":\"$name\",\"shell\":\"fish\",\"pid\":$_TERMINAL_REMOTE_PID}"

  # Background process: send registration and keep connection open
  # The 'cat' blocks forever, keeping nc alive until killed
  begin
    echo $msg
    cat  # Block to keep connection open
  end | nc -U "$_TERMINAL_REMOTE_SOCKET" 2>/dev/null &
  set -g _TERMINAL_REMOTE_BG_PID (jobs -lp | tail -1)
  disown $_TERMINAL_REMOTE_BG_PID 2>/dev/null

  # Give it a moment to connect
  sleep 0.05

  # Check if it's still running (successful connection)
  if kill -0 $_TERMINAL_REMOTE_BG_PID 2>/dev/null
    set -g _TERMINAL_REMOTE_CONNECTED 1
    set -g _TERMINAL_REMOTE_WARNED 0
  else
    set -g _TERMINAL_REMOTE_CONNECTED 0
    set -g _TERMINAL_REMOTE_BG_PID ""
  end
end

# Send directory rename update (fire-and-forget)
function _terminal_remote_send_update
  set -l name (_terminal_remote_json_escape (_terminal_remote_session_name))
  set -l msg "{\"type\":\"rename\",\"name\":\"$name\"}"

  # Send via a quick connection, don't wait for response
  echo $msg | nc -U "$_TERMINAL_REMOTE_SOCKET" 2>/dev/null &
  disown (jobs -lp | tail -1) 2>/dev/null
end

# Directory change hook (fires when PWD changes)
function _terminal_remote_chpwd --on-variable PWD
  test $_TERMINAL_REMOTE_CONNECTED -eq 1; or return
  _terminal_remote_send_update
end

# Check connection and handle reconnection (called on each prompt)
function _terminal_remote_prompt_check --on-event fish_prompt
  # Check if background process died (disconnect)
  if test $_TERMINAL_REMOTE_CONNECTED -eq 1 -a -n "$_TERMINAL_REMOTE_BG_PID"
    if not kill -0 $_TERMINAL_REMOTE_BG_PID 2>/dev/null
      set -g _TERMINAL_REMOTE_CONNECTED 0
      set -g _TERMINAL_REMOTE_BG_PID ""
      if test $_TERMINAL_REMOTE_WARNED -eq 0
        echo "Terminal Remote disconnected"
        set -g _TERMINAL_REMOTE_WARNED 1
      end
      _terminal_remote_start_watcher
    end
  end

  # Try to reconnect if socket appeared and we're not connected
  if test $_TERMINAL_REMOTE_CONNECTED -eq 0 -a -S "$_TERMINAL_REMOTE_SOCKET"
    _terminal_remote_connect
    if test $_TERMINAL_REMOTE_CONNECTED -eq 1
      echo "Connected to Terminal Remote"
    end
  end
end

# Background watcher for auto-reconnect when mac-client starts
function _terminal_remote_start_watcher
  # Already watching?
  if test -n "$_TERMINAL_REMOTE_WATCHER_PID"
    if kill -0 $_TERMINAL_REMOTE_WATCHER_PID 2>/dev/null
      return
    end
  end

  # Fish subprocess isolation means we can't easily signal back
  # Instead, rely on fish_prompt hook to check socket and reconnect
  # The watcher just provides a background check if shell is idle
  fish -c '
    while true
      sleep 5
      if test -S "'$_TERMINAL_REMOTE_SOCKET'"
        # Socket appeared, can exit - prompt hook handles reconnection
        break
      end
    end
  ' &
  set -g _TERMINAL_REMOTE_WATCHER_PID (jobs -lp | tail -1)
  disown $_TERMINAL_REMOTE_WATCHER_PID 2>/dev/null
end

# Cleanup on shell exit
function _terminal_remote_cleanup --on-event fish_exit
  # Kill background process holding socket
  if test -n "$_TERMINAL_REMOTE_BG_PID"
    kill $_TERMINAL_REMOTE_BG_PID 2>/dev/null
  end
  # Kill watcher
  if test -n "$_TERMINAL_REMOTE_WATCHER_PID"
    kill $_TERMINAL_REMOTE_WATCHER_PID 2>/dev/null
  end
end

# Initialize connection on script load
function _terminal_remote_init
  # Fast check: socket file exists?
  if not test -S "$_TERMINAL_REMOTE_SOCKET"
    echo "Terminal Remote not running"
    _terminal_remote_start_watcher
    return 0
  end

  _terminal_remote_connect
  if test $_TERMINAL_REMOTE_CONNECTED -eq 1
    echo "Connected to Terminal Remote"
  else
    echo "Terminal Remote not running"
    _terminal_remote_start_watcher
  end
end

# Auto-initialize when sourced
_terminal_remote_init
