/**
 * iTerm2 Remote - Mac Client Entry Point
 *
 * Connects to the relay server, receives a session code,
 * and displays it for the user to enter in their browser.
 * Integrates the iTerm2 Python bridge via SessionManager
 * for terminal I/O routing.
 */

import { ConnectionManager } from './connection.js';
import { SessionManager } from './session-manager.js';

const RELAY_URL = process.env.RELAY_URL || 'ws://localhost:8080/mac';

console.log('iTerm2 Remote - Mac Client');
console.log('==========================');
console.log(`Connecting to relay: ${RELAY_URL}`);
console.log();

/**
 * Display session code in a prominent box.
 */
function displaySessionCode(code: string): void {
  console.log();
  console.log('+--------------------------------------+');
  console.log('|                                      |');
  console.log(`|     Session Code: ${code}          |`);
  console.log('|                                      |');
  console.log('|  Enter this code in your browser     |');
  console.log('|  to connect to this terminal.        |');
  console.log('|                                      |');
  console.log('+--------------------------------------+');
  console.log();
}

// Create the session manager that routes terminal I/O between
// the iTerm2 Python bridge and the relay WebSocket connection.
const sessionManager = new SessionManager((message) => {
  manager.send(message);
});

const manager = new ConnectionManager(RELAY_URL, {
  onCodeReceived: (code) => {
    displaySessionCode(code);
  },
  onStateChange: (state) => {
    if (state === 'connected') {
      console.log('[Status] Ready for browser connections');
      // Start the iTerm2 bridge once connected to relay
      sessionManager.start().catch((err) => {
        console.error('[Error] Failed to start iTerm2 bridge:', err);
      });
    }
  },
  onMessage: (data) => {
    sessionManager.handleRelayMessage(data);
  },
  onError: (error) => {
    console.error('[Error]', error.message);
  },
});

// Handle graceful shutdown
async function shutdown(): Promise<void> {
  console.log('\nShutting down...');
  await sessionManager.stop();
  manager.disconnect();
  process.exit(0);
}

process.on('SIGINT', () => {
  shutdown().catch(() => process.exit(1));
});

process.on('SIGTERM', () => {
  shutdown().catch(() => process.exit(1));
});

// Handle uncaught errors
process.on('uncaughtException', (err) => {
  console.error('[Fatal] Uncaught exception:', err);
  sessionManager.stop().finally(() => {
    manager.disconnect();
    process.exit(1);
  });
});

process.on('unhandledRejection', (reason) => {
  console.error('[Fatal] Unhandled rejection:', reason);
  sessionManager.stop().finally(() => {
    manager.disconnect();
    process.exit(1);
  });
});

// Start connection
manager.connect();
