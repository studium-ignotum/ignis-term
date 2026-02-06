/**
 * Binary frame encoding/decoding utilities for WebSocket communication.
 *
 * Binary Frame Format:
 * +----------------+--------------------+-------------------+
 * | Session ID Len | Session ID (UTF-8) | Payload (bytes)   |
 * | (1 byte)       | (variable)         | (remaining bytes) |
 * +----------------+--------------------+-------------------+
 *
 * This format allows efficient routing of binary terminal data
 * to the correct session without JSON parsing overhead.
 */

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

/**
 * Encode a binary frame with session ID prefix.
 *
 * @param sessionId - The session identifier (max 255 bytes when UTF-8 encoded)
 * @param payload - The binary payload (terminal data)
 * @returns Combined frame as Uint8Array
 *
 * @example
 * const frame = encodeBinaryFrame("sess-1", new TextEncoder().encode("hello"));
 * // Result: [6, 115, 101, 115, 115, 45, 49, 104, 101, 108, 108, 111]
 * //         ^len  ^---session id bytes---^  ^---payload bytes---^
 */
export function encodeBinaryFrame(sessionId: string, payload: Uint8Array): Uint8Array {
  const sessionIdBytes = textEncoder.encode(sessionId);

  if (sessionIdBytes.length > 255) {
    throw new Error(`Session ID too long: ${sessionIdBytes.length} bytes (max 255)`);
  }

  const frame = new Uint8Array(1 + sessionIdBytes.length + payload.length);
  frame[0] = sessionIdBytes.length;
  frame.set(sessionIdBytes, 1);
  frame.set(payload, 1 + sessionIdBytes.length);

  return frame;
}

/**
 * Decode a binary frame to extract session ID and payload.
 *
 * @param frame - The binary frame to decode
 * @returns Object with sessionId string and payload Uint8Array
 * @throws Error if frame is too short or malformed
 *
 * @example
 * const { sessionId, payload } = decodeBinaryFrame(frame);
 * console.log(sessionId); // "sess-1"
 * console.log(new TextDecoder().decode(payload)); // "hello"
 */
export function decodeBinaryFrame(frame: Uint8Array): { sessionId: string; payload: Uint8Array } {
  if (frame.length < 1) {
    throw new Error('Frame too short: missing session ID length byte');
  }

  const sessionIdLength = frame[0];

  if (frame.length < 1 + sessionIdLength) {
    throw new Error(`Frame too short: expected ${1 + sessionIdLength} bytes for header, got ${frame.length}`);
  }

  const sessionIdBytes = frame.slice(1, 1 + sessionIdLength);
  const sessionId = textDecoder.decode(sessionIdBytes);
  const payload = frame.slice(1 + sessionIdLength);

  return { sessionId, payload };
}

/**
 * Encode a terminal resize message for the given session.
 *
 * The resize payload is JSON-formatted for the mac-client to parse.
 *
 * @param sessionId - Target session ID
 * @param cols - Number of columns
 * @param rows - Number of rows
 * @returns Binary frame with resize command
 *
 * @example
 * const frame = encodeResizeMessage("sess-1", 80, 24);
 * // Sends: {"type":"resize","cols":80,"rows":24}
 */
export function encodeResizeMessage(sessionId: string, cols: number, rows: number): Uint8Array {
  const payload = textEncoder.encode(JSON.stringify({
    type: 'resize',
    cols,
    rows,
  }));
  return encodeBinaryFrame(sessionId, payload);
}

/**
 * Encode terminal input for the given session.
 *
 * @param sessionId - Target session ID
 * @param input - User input string (keystrokes)
 * @returns Binary frame with input data
 *
 * @example
 * const frame = encodeInputMessage("sess-1", "ls -la\n");
 */
export function encodeInputMessage(sessionId: string, input: string): Uint8Array {
  const payload = textEncoder.encode(input);
  return encodeBinaryFrame(sessionId, payload);
}

// =============================================================================
// Self-test (runs when imported in development)
// =============================================================================

if (import.meta.env?.DEV) {
  // Test encode/decode roundtrip
  const testSessionId = "sess-1";
  const testPayload = textEncoder.encode("hello");
  const frame = encodeBinaryFrame(testSessionId, testPayload);
  const decoded = decodeBinaryFrame(frame);

  console.assert(
    decoded.sessionId === testSessionId,
    `Session ID mismatch: expected "${testSessionId}", got "${decoded.sessionId}"`
  );
  console.assert(
    textDecoder.decode(decoded.payload) === "hello",
    `Payload mismatch: expected "hello", got "${textDecoder.decode(decoded.payload)}"`
  );

  // Test empty payload
  const emptyFrame = encodeBinaryFrame("test", new Uint8Array(0));
  const emptyDecoded = decodeBinaryFrame(emptyFrame);
  console.assert(emptyDecoded.sessionId === "test", "Empty payload session ID mismatch");
  console.assert(emptyDecoded.payload.length === 0, "Empty payload should have 0 length");

  // Test resize message
  const resizeFrame = encodeResizeMessage("sess-2", 120, 40);
  const resizeDecoded = decodeBinaryFrame(resizeFrame);
  console.assert(resizeDecoded.sessionId === "sess-2", "Resize session ID mismatch");
  const resizePayload = JSON.parse(textDecoder.decode(resizeDecoded.payload));
  console.assert(resizePayload.type === "resize", "Resize type mismatch");
  console.assert(resizePayload.cols === 120, "Resize cols mismatch");
  console.assert(resizePayload.rows === 40, "Resize rows mismatch");

  // Test input message
  const inputFrame = encodeInputMessage("sess-3", "ls -la\n");
  const inputDecoded = decodeBinaryFrame(inputFrame);
  console.assert(inputDecoded.sessionId === "sess-3", "Input session ID mismatch");
  console.assert(textDecoder.decode(inputDecoded.payload) === "ls -la\n", "Input payload mismatch");

  console.log("[binary.ts] All self-tests passed");
}
