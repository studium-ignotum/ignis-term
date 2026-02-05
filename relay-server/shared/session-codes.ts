/**
 * Session code generation using nanoid with human-readable alphabet
 */

import { customAlphabet } from 'nanoid';
import { SESSION_CODE_ALPHABET, SESSION_CODE_LENGTH } from './constants.js';

/**
 * Generates a cryptographically secure session code.
 * Uses nolookalikes alphabet to avoid ambiguous characters.
 * Example output: "H4F7KN"
 */
export const generateSessionCode = customAlphabet(
  SESSION_CODE_ALPHABET,
  SESSION_CODE_LENGTH
);
