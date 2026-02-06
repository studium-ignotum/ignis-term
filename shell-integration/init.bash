# Terminal Remote - Bash Integration
# Connects shell sessions to mac-client for remote access
# Source this file in .bashrc: source ~/.terminal-remote/init.bash

# Configuration
_TERMINAL_REMOTE_SOCKET="/tmp/terminal-remote.sock"
_TERMINAL_REMOTE_CONNECTED=0
_TERMINAL_REMOTE_BG_PID=""
_TERMINAL_REMOTE_WATCHER_PID=""
_TERMINAL_REMOTE_WARNED=0
_TERMINAL_REMOTE_PREVPWD="$PWD"

# Generate session name: "dirname [PID]"
_terminal_remote_session_name() {
  echo "${PWD##*/} [$$]"
}

# Escape string for JSON (backslash, quote, newline, tab)
_terminal_remote_json_escape() {
  local str="$1"
  str="${str//\\/\\\\}"    # Escape backslashes first
  str="${str//\"/\\\"}"    # Escape quotes
  str="${str//$'\n'/\\n}"  # Escape newlines
  str="${str//$'\t'/\\t}"  # Escape tabs
  echo "$str"
}

# Attempt to connect to Terminal Remote
_terminal_remote_connect() {
  local name
  name=$(_terminal_remote_json_escape "$(_terminal_remote_session_name)")
  local msg="{\"name\":\"${name}\",\"shell\":\"bash\",\"pid\":$$}"

  # Background process: send registration and keep connection open
  # The 'cat' blocks forever, keeping nc alive until killed
  (
    echo "$msg"
    cat  # Block to keep connection open
  ) | nc -U "$_TERMINAL_REMOTE_SOCKET" 2>/dev/null &
  _TERMINAL_REMOTE_BG_PID=$!
  disown "$_TERMINAL_REMOTE_BG_PID" 2>/dev/null

  # Give it a moment to connect
  sleep 0.05

  # Check if it's still running (successful connection)
  if kill -0 "$_TERMINAL_REMOTE_BG_PID" 2>/dev/null; then
    _TERMINAL_REMOTE_CONNECTED=1
    _TERMINAL_REMOTE_WARNED=0
  else
    _TERMINAL_REMOTE_CONNECTED=0
    _TERMINAL_REMOTE_BG_PID=""
  fi
}

# Send directory rename update (fire-and-forget)
_terminal_remote_send_update() {
  local name
  name=$(_terminal_remote_json_escape "$(_terminal_remote_session_name)")
  local msg="{\"type\":\"rename\",\"name\":\"${name}\"}"

  # Send via a quick connection, don't wait for response
  echo "$msg" | nc -U "$_TERMINAL_REMOTE_SOCKET" 2>/dev/null &
  disown $! 2>/dev/null
}

# Directory change detection (called in PROMPT_COMMAND)
_terminal_remote_chpwd_hook() {
  if [[ "$_TERMINAL_REMOTE_PREVPWD" != "$PWD" ]]; then
    _TERMINAL_REMOTE_PREVPWD="$PWD"
    if [[ $_TERMINAL_REMOTE_CONNECTED -eq 1 ]]; then
      _terminal_remote_send_update
    fi
  fi
}

# Check if connection is still alive (called in PROMPT_COMMAND)
_terminal_remote_check_connection() {
  if [[ $_TERMINAL_REMOTE_CONNECTED -eq 1 ]] && [[ -n "$_TERMINAL_REMOTE_BG_PID" ]]; then
    if ! kill -0 "$_TERMINAL_REMOTE_BG_PID" 2>/dev/null; then
      _TERMINAL_REMOTE_CONNECTED=0
      _TERMINAL_REMOTE_BG_PID=""
      if [[ $_TERMINAL_REMOTE_WARNED -eq 0 ]]; then
        echo "Terminal Remote disconnected"
        _TERMINAL_REMOTE_WARNED=1
      fi
      _terminal_remote_start_watcher
    fi
  fi

  # Also try to reconnect if socket appeared and we're not connected
  if [[ $_TERMINAL_REMOTE_CONNECTED -eq 0 ]] && [[ -S "$_TERMINAL_REMOTE_SOCKET" ]]; then
    _terminal_remote_connect
    if [[ $_TERMINAL_REMOTE_CONNECTED -eq 1 ]]; then
      echo "Connected to Terminal Remote"
    fi
  fi
}

# Background watcher for auto-reconnect when mac-client starts
_terminal_remote_start_watcher() {
  # Already watching?
  if [[ -n "$_TERMINAL_REMOTE_WATCHER_PID" ]] && kill -0 "$_TERMINAL_REMOTE_WATCHER_PID" 2>/dev/null; then
    return
  fi

  (
    while true; do
      sleep 5
      # Check if socket appeared
      if [[ -S "$_TERMINAL_REMOTE_SOCKET" ]]; then
        # Socket exists, watcher can stop - prompt hook will handle reconnection
        break
      fi
    done
  ) &
  _TERMINAL_REMOTE_WATCHER_PID=$!
  disown "$_TERMINAL_REMOTE_WATCHER_PID" 2>/dev/null
}

# Combined PROMPT_COMMAND hook
_terminal_remote_prompt_hook() {
  _terminal_remote_check_connection
  _terminal_remote_chpwd_hook
}

# Cleanup on shell exit
_terminal_remote_cleanup() {
  # Kill background process holding socket
  [[ -n "$_TERMINAL_REMOTE_BG_PID" ]] && kill "$_TERMINAL_REMOTE_BG_PID" 2>/dev/null
  # Kill watcher
  [[ -n "$_TERMINAL_REMOTE_WATCHER_PID" ]] && kill "$_TERMINAL_REMOTE_WATCHER_PID" 2>/dev/null
}

# Initialize connection on script load
_terminal_remote_init() {
  # Fast check: socket file exists?
  if [[ ! -S "$_TERMINAL_REMOTE_SOCKET" ]]; then
    echo "Terminal Remote not running"
    _terminal_remote_start_watcher
    return 0
  fi

  _terminal_remote_connect
  if [[ $_TERMINAL_REMOTE_CONNECTED -eq 1 ]]; then
    echo "Connected to Terminal Remote"
  else
    echo "Terminal Remote not running"
    _terminal_remote_start_watcher
  fi
}

# Append to PROMPT_COMMAND safely (preserve existing commands)
PROMPT_COMMAND="_terminal_remote_prompt_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"

# Register exit handler
trap '_terminal_remote_cleanup' EXIT

# Auto-initialize when sourced
_terminal_remote_init
