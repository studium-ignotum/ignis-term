//! pty-proxy: Transparent PTY proxy for terminal session capture.
//!
//! Sits between terminal emulator and shell:
//!   Terminal (iTerm2, etc.) <-> pty-proxy <-> Shell (zsh/bash)
//!
//! All I/O is forwarded transparently. A copy of the raw byte stream
//! is sent to mac-client via Unix socket for remote browser access.
//!
//! The terminal emulator sees a normal PTY — no scroll/copy/mouse conflicts.

use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::libc::{STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::pty::{openpty, OpenptyResult};
use nix::sys::signal::{self, SigHandler, Signal};
use nix::sys::termios::{self, SetArg};
use nix::sys::uio::writev;
use std::io::IoSlice;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{close, dup2, execvp, fork, read, setsid, write, ForkResult, Pid};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, IntoRawFd, OwnedFd, RawFd};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Instant;

const SOCKET_PATH: &str = "/tmp/terminal-remote.sock";
const BUF_SIZE: usize = 8192;
const RECONNECT_INTERVAL_SECS: u64 = 5;

/// Registration message sent to mac-client on connect.
#[derive(Serialize)]
struct Registration {
    name: String,
    shell: String,
    pid: u32,
    tty: String,
    proxy_version: u8,
}

/// Control messages received from mac-client.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControlMessage {
    /// Input from browser to inject into shell
    Input { data: Vec<u8> },
    /// Resize request from browser
    Resize { cols: u16, rows: u16 },
    /// Close session — kill child and exit cleanly (code 0)
    Close,
}

// Global state for signal handlers
static CHILD_PID: AtomicI32 = AtomicI32::new(0);
static CHILD_EXITED: AtomicBool = AtomicBool::new(false);
static SIGWINCH_RECEIVED: AtomicBool = AtomicBool::new(false);

/// SIGCHLD handler — child shell exited.
extern "C" fn handle_sigchld(_sig: i32) {
    CHILD_EXITED.store(true, Ordering::Relaxed);
}

/// SIGWINCH handler — terminal resized.
/// We need to forward this to the child PTY.
extern "C" fn handle_sigwinch(_sig: i32) {
    SIGWINCH_RECEIVED.store(true, Ordering::Relaxed);
}

fn main() {
    // Determine shell to exec
    let shell = detect_shell();

    // Save original terminal state for restore on exit
    let orig_termios = termios::tcgetattr(unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) }).ok();

    // Open PTY pair
    let OpenptyResult { master, slave } = openpty(None, None).unwrap_or_else(|e| {
        eprintln!("pty-proxy: openpty failed: {}", e);
        fallback_exec_shell(&shell);
    });

    let master_fd = master.as_raw_fd();
    let slave_fd = slave.as_raw_fd();

    // FIX #6: Set terminal size on the slave PTY BEFORE fork
    // so the child shell inherits correct dimensions immediately.
    if let Some(size) = get_terminal_size(STDIN_FILENO) {
        set_pty_size(slave_fd, &size);
    }

    // Fork
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            // === CHILD: becomes the shell ===
            drop(master); // close master in child

            // Create new session (detach from controlling terminal)
            setsid().ok();

            // Set slave as controlling terminal
            unsafe { libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) };

            // Redirect stdio to slave PTY
            dup2(slave_fd, STDIN_FILENO).unwrap();
            dup2(slave_fd, STDOUT_FILENO).unwrap();
            dup2(slave_fd, STDERR_FILENO).unwrap();

            if slave_fd > STDERR_FILENO {
                drop(slave); // close original fd
            } else {
                // FIX #5: Use into_raw_fd() instead of mem::forget to avoid fd leak risk.
                // The fd IS our stdio now, so we release ownership without closing.
                let _ = slave.into_raw_fd();
            }

            // Mark that we're inside the proxy (prevent recursion in .zshrc)
            std::env::set_var("PTY_PROXY_ACTIVE", "1");

            // Set TERM if not set
            if std::env::var("TERM").is_err() {
                std::env::set_var("TERM", "xterm-256color");
            }

            // Exec the shell
            let shell_cstr = CString::new(shell.as_str()).unwrap();
            let args = [shell_cstr.clone()];
            execvp(&shell_cstr, &args).unwrap_or_else(|e| {
                eprintln!("pty-proxy: exec {} failed: {}", shell, e);
                std::process::exit(1);
            });
        }
        Ok(ForkResult::Parent { child }) => {
            // === PARENT: proxy I/O ===
            drop(slave); // close slave in parent

            CHILD_PID.store(child.as_raw() as i32, Ordering::Relaxed);

            // Also set size on master (belt and suspenders — slave already has it)
            if let Some(size) = get_terminal_size(STDIN_FILENO) {
                set_pty_size(master_fd, &size);
            }

            // Install signal handlers
            unsafe {
                signal::signal(Signal::SIGCHLD, SigHandler::Handler(handle_sigchld)).ok();
                signal::signal(Signal::SIGWINCH, SigHandler::Handler(handle_sigwinch)).ok();
                // Ignore SIGPIPE (socket writes may fail)
                signal::signal(Signal::SIGPIPE, SigHandler::SigIgn).ok();
            }

            // Put terminal in raw mode (pass everything through)
            if let Some(ref orig) = orig_termios {
                let mut raw = orig.clone();
                termios::cfmakeraw(&mut raw);
                termios::tcsetattr(
                    unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) },
                    SetArg::TCSANOW,
                    &raw,
                )
                .ok();
            }

            // Try to connect to mac-client
            let socket_fd = connect_to_mac_client(&shell, child);

            // Set master to non-blocking
            set_nonblocking(master_fd);
            if let Some(ref fd) = socket_fd {
                set_nonblocking(fd.as_raw_fd());
            }

            // Main I/O loop
            let exit_code = proxy_loop(master_fd, socket_fd, child, &shell);

            // Restore terminal
            if let Some(ref orig) = orig_termios {
                termios::tcsetattr(
                    unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) },
                    SetArg::TCSANOW,
                    orig,
                )
                .ok();
            }

            std::process::exit(exit_code);
        }
        Err(e) => {
            eprintln!("pty-proxy: fork failed: {}", e);
            // Restore terminal and fallback
            if let Some(ref orig) = orig_termios {
                termios::tcsetattr(
                    unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) },
                    SetArg::TCSANOW,
                    orig,
                )
                .ok();
            }
            fallback_exec_shell(&shell);
        }
    }
}

/// Main proxy loop. Returns exit code.
/// FIX #2 & #4: socket_fd is now mutable (Option<OwnedFd>) so we can reconnect.
fn proxy_loop(master_fd: RawFd, mut socket_fd: Option<OwnedFd>, child: Pid, shell: &str) -> i32 {
    let mut buf = [0u8; BUF_SIZE];

    // Buffer for incoming data from mac-client (browser input)
    let mut socket_buf = [0u8; BUF_SIZE];

    // Frame buffer for length-prefixed messages from mac-client
    let mut frame_buf: Vec<u8> = Vec::with_capacity(BUF_SIZE);

    // Reconnect tracking
    let mut last_reconnect_attempt: Option<Instant> = None;

    loop {
        // Check if child exited
        if CHILD_EXITED.load(Ordering::Relaxed) {
            return reap_child(child);
        }

        // Handle SIGWINCH — forward terminal resize to child PTY
        if SIGWINCH_RECEIVED.swap(false, Ordering::Relaxed) {
            if let Some(size) = get_terminal_size(STDIN_FILENO) {
                set_pty_size(master_fd, &size);
                // Also notify mac-client about resize
                if let Some(ref sock) = socket_fd {
                    let resize_msg = format!(
                        "{{\"type\":\"resize\",\"cols\":{},\"rows\":{}}}",
                        size.ws_col, size.ws_row
                    );
                    send_frame(sock.as_raw_fd(), resize_msg.as_bytes());
                }
            }
        }

        // FIX #2: Try to reconnect if socket is gone
        if socket_fd.is_none() {
            let should_try = match last_reconnect_attempt {
                None => true,
                Some(t) => t.elapsed().as_secs() >= RECONNECT_INTERVAL_SECS,
            };
            if should_try {
                last_reconnect_attempt = Some(Instant::now());
                if let Some(fd) = connect_to_mac_client(shell, child) {
                    set_nonblocking(fd.as_raw_fd());
                    socket_fd = Some(fd);
                    frame_buf.clear(); // reset frame buffer for new connection
                }
            }
        }

        // Build poll fds
        let mut poll_fds = vec![
            PollFd::new(
                unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) },
                PollFlags::POLLIN,
            ),
            PollFd::new(
                unsafe { BorrowedFd::borrow_raw(master_fd) },
                PollFlags::POLLIN,
            ),
        ];
        if let Some(ref sock) = socket_fd {
            poll_fds.push(PollFd::new(
                unsafe { BorrowedFd::borrow_raw(sock.as_raw_fd()) },
                PollFlags::POLLIN,
            ));
        }

        // Poll with 100ms timeout (to check signals)
        match poll(&mut poll_fds, PollTimeout::from(100u16)) {
            Ok(0) => continue, // timeout
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => {
                eprintln!("pty-proxy: poll error: {}", e);
                break;
            }
            Ok(_) => {}
        }

        // stdin → master PTY (user typing in terminal)
        if let Some(revents) = poll_fds[0].revents() {
            if revents.contains(PollFlags::POLLIN) {
                match read(STDIN_FILENO, &mut buf) {
                    Ok(0) => break, // stdin closed
                    Ok(n) => {
                        // Write to shell
                        write_all(master_fd, &buf[..n]);
                        // Tee input to mac-client (tagged as input)
                        if let Some(ref sock) = socket_fd {
                            let mut msg = Vec::with_capacity(1 + n);
                            msg.push(b'I'); // 'I' = input
                            msg.extend_from_slice(&buf[..n]);
                            send_frame(sock.as_raw_fd(), &msg);
                        }
                    }
                    Err(nix::errno::Errno::EAGAIN | nix::errno::Errno::EINTR) => {}
                    Err(_) => break,
                }
            }
            if revents.contains(PollFlags::POLLHUP) {
                break;
            }
        }

        // master PTY → stdout (shell output to terminal)
        if let Some(revents) = poll_fds[1].revents() {
            if revents.contains(PollFlags::POLLIN) {
                match read(master_fd, &mut buf) {
                    Ok(0) => break, // PTY closed (child exited)
                    Ok(n) => {
                        // Write to terminal
                        write_all(STDOUT_FILENO, &buf[..n]);
                        // Tee output to mac-client
                        if let Some(ref sock) = socket_fd {
                            let mut msg = Vec::with_capacity(1 + n);
                            msg.push(b'O'); // 'O' = output
                            msg.extend_from_slice(&buf[..n]);
                            send_frame(sock.as_raw_fd(), &msg);
                        }
                    }
                    Err(nix::errno::Errno::EAGAIN | nix::errno::Errno::EINTR) => {}
                    Err(_) => break,
                }
            }
            if revents.contains(PollFlags::POLLHUP) {
                // Shell closed PTY — will get SIGCHLD soon
                // Drain any remaining output
                loop {
                    match read(master_fd, &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            write_all(STDOUT_FILENO, &buf[..n]);
                            if let Some(ref sock) = socket_fd {
                                let mut msg = Vec::with_capacity(1 + n);
                                msg.push(b'O');
                                msg.extend_from_slice(&buf[..n]);
                                send_frame(sock.as_raw_fd(), &msg);
                            }
                        }
                    }
                }
                break;
            }
        }

        // mac-client socket → master PTY (browser input injection)
        // FIX #4: Handle socket disconnect by setting socket_fd = None
        if socket_fd.is_some() && poll_fds.len() > 2 {
            if let Some(revents) = poll_fds[2].revents() {
                let disconnect = revents.contains(PollFlags::POLLHUP)
                    || revents.contains(PollFlags::POLLERR);

                // Read pending data even on POLLHUP — the close message may be buffered
                if revents.contains(PollFlags::POLLIN) {
                    let sock_raw = socket_fd.as_ref().unwrap().as_raw_fd();
                    match read(sock_raw, &mut socket_buf) {
                        Ok(0) => {
                            // mac-client disconnected — drop socket, will reconnect
                            socket_fd = None;
                            frame_buf.clear();
                        }
                        Ok(n) => {
                            frame_buf.extend_from_slice(&socket_buf[..n]);
                            // Process complete frames (4-byte length prefix + payload)
                            while frame_buf.len() >= 4 {
                                let len = u32::from_be_bytes([
                                    frame_buf[0],
                                    frame_buf[1],
                                    frame_buf[2],
                                    frame_buf[3],
                                ]) as usize;
                                if frame_buf.len() < 4 + len {
                                    break; // incomplete frame
                                }
                                let payload = frame_buf[4..4 + len].to_vec();
                                frame_buf.drain(..4 + len);
                                if handle_mac_client_message(&payload, master_fd, child) {
                                    // Close requested — wait for child and exit with 0
                                    reap_child(child);
                                    return 0;
                                }
                            }
                        }
                        Err(nix::errno::Errno::EAGAIN | nix::errno::Errno::EINTR) => {}
                        Err(_) => {
                            // socket error, drop and reconnect
                            socket_fd = None;
                            frame_buf.clear();
                        }
                    }
                }

                if disconnect {
                    socket_fd = None;
                    frame_buf.clear();
                }
            }
        }
    }

    reap_child(child)
}

/// Handle a message from mac-client (browser → shell).
/// Returns true if pty-proxy should exit cleanly (Close message received).
fn handle_mac_client_message(payload: &[u8], master_fd: RawFd, child: Pid) -> bool {
    // Try JSON parse first
    if let Ok(msg) = serde_json::from_slice::<ControlMessage>(payload) {
        match msg {
            ControlMessage::Input { data } => {
                write_all(master_fd, &data);
            }
            ControlMessage::Resize { cols, rows } => {
                let size = libc::winsize {
                    ws_row: rows,
                    ws_col: cols,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };
                set_pty_size(master_fd, &size);
            }
            ControlMessage::Close => {
                // Kill child shell — use SIGHUP, not SIGTERM.
                // zsh ignores SIGTERM in interactive mode, but respects SIGHUP.
                unsafe { libc::kill(child.as_raw() as i32, libc::SIGHUP); }
                return true;
            }
        }
    }
    // If not JSON, treat as raw input
    else {
        write_all(master_fd, payload);
    }
    false
}

/// Connect to mac-client via Unix socket. Returns None on failure (non-fatal).
fn connect_to_mac_client(shell: &str, child_pid: Pid) -> Option<OwnedFd> {
    use std::os::unix::net::UnixStream;

    let stream = match UnixStream::connect(SOCKET_PATH) {
        Ok(s) => s,
        Err(_) => return None, // mac-client not running, that's OK
    };

    // FIX #5: Use into_raw_fd() instead of mem::forget to properly transfer ownership.
    let fd = stream.into_raw_fd();

    // Send registration as length-prefixed JSON
    let tty_name = std::env::var("TTY")
        .or_else(|_| {
            // Try ttyname on stdin
            nix::unistd::ttyname(unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) })
                .ok()
                .map(|p| p.to_string_lossy().to_string())
                .ok_or(std::env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "unknown".to_string());

    let reg = Registration {
        name: format!(
            "{} - {}",
            shell,
            std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "~".to_string())
        ),
        shell: shell.to_string(),
        pid: child_pid.as_raw() as u32,
        tty: tty_name,
        proxy_version: 1,
    };

    let json = match serde_json::to_vec(&reg) {
        Ok(j) => j,
        Err(_) => {
            close(fd).ok();
            return None;
        }
    };

    // Send: 4-byte length (big-endian) + JSON
    send_frame(fd, &json);

    // Also send initial terminal size
    if let Some(size) = get_terminal_size(STDIN_FILENO) {
        let resize_msg = format!(
            "{{\"type\":\"resize\",\"cols\":{},\"rows\":{}}}",
            size.ws_col, size.ws_row
        );
        send_frame(fd, resize_msg.as_bytes());
    }

    Some(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Send a length-prefixed frame atomically: 4 bytes big-endian length + payload.
/// FIX #1: Use writev() for atomic writes — length prefix and payload in a single syscall.
fn send_frame(fd: RawFd, data: &[u8]) {
    let len = (data.len() as u32).to_be_bytes();
    let iov = [IoSlice::new(&len), IoSlice::new(data)];
    // Best-effort write, ignore errors (socket may be gone)
    let _ = writev(unsafe { BorrowedFd::borrow_raw(fd) }, &iov);
}

/// Write all bytes to fd, retrying on EINTR/EAGAIN.
fn write_all(fd: RawFd, mut data: &[u8]) {
    while !data.is_empty() {
        match write(unsafe { BorrowedFd::borrow_raw(fd) }, data) {
            Ok(n) => data = &data[n..],
            Err(nix::errno::Errno::EINTR) => continue,
            Err(nix::errno::Errno::EAGAIN) => {
                // Non-blocking fd is full, yield briefly
                std::thread::sleep(std::time::Duration::from_micros(100));
                continue;
            }
            Err(_) => break,
        }
    }
}

fn set_nonblocking(fd: RawFd) {
    if let Ok(flags) = fcntl(fd, FcntlArg::F_GETFL) {
        let new_flags = OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK;
        let _ = fcntl(fd, FcntlArg::F_SETFL(new_flags));
    }
}

fn get_terminal_size(fd: RawFd) -> Option<libc::winsize> {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    let ret = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut size) };
    if ret == 0 {
        Some(size)
    } else {
        None
    }
}

fn set_pty_size(fd: RawFd, size: &libc::winsize) {
    unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, size) };
}

fn reap_child(child: Pid) -> i32 {
    match waitpid(child, None) {
        Ok(WaitStatus::Exited(_, code)) => code,
        Ok(WaitStatus::Signaled(_, sig, _)) => 128 + sig as i32,
        _ => 1,
    }
}

fn detect_shell() -> String {
    // Check SHELL env var
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.is_empty() {
            return shell;
        }
    }
    // Fallback
    "/bin/zsh".to_string()
}

/// If anything goes wrong, just exec the shell directly.
/// User gets a normal shell without proxy — no harm done.
fn fallback_exec_shell(shell: &str) -> ! {
    eprintln!("pty-proxy: falling back to direct shell exec");
    std::env::set_var("PTY_PROXY_ACTIVE", "1"); // prevent recursion
    let shell_cstr = CString::new(shell).unwrap();
    let args = [shell_cstr.clone()];
    execvp(&shell_cstr, &args).unwrap();
    std::process::exit(1);
}
