/**
 * Floating special keys bar for mobile/touch devices.
 *
 * Provides buttons for keys that are hard or impossible to type on a
 * mobile software keyboard: Esc, Tab, Ctrl+key, Alt/Meta, pipe, tilde,
 * and arrow keys.
 *
 * Ctrl and Alt are "sticky" — press once to activate for the next key,
 * press again to deactivate.
 */

import { useState } from 'react';
import './MobileControlBar.css';

interface MobileControlBarProps {
  onKey: (data: string) => void;
}

export default function MobileControlBar({ onKey }: MobileControlBarProps) {
  const [ctrlActive, setCtrlActive] = useState(false);
  const [altActive, setAltActive] = useState(false);

  function sendKey(key: string) {
    if (ctrlActive) {
      if (key.length === 1 && key >= 'A' && key <= 'Z') {
        onKey(String.fromCharCode(key.charCodeAt(0) - 64));
      } else if (key.length === 1 && key >= 'a' && key <= 'z') {
        onKey(String.fromCharCode(key.charCodeAt(0) - 96));
      } else {
        onKey(key);
      }
      setCtrlActive(false);
    } else if (altActive) {
      onKey('\x1b' + key);
      setAltActive(false);
    } else {
      onKey(key);
    }
  }

  function toggleCtrl() {
    setCtrlActive((prev) => {
      if (!prev) setAltActive(false);
      return !prev;
    });
  }

  function toggleAlt() {
    setAltActive((prev) => {
      if (!prev) setCtrlActive(false);
      return !prev;
    });
  }

  return (
    <div className="mobile-control-bar">
      <button className="key-btn" onClick={() => sendKey('\x1b')} title="Escape">Esc</button>
      <button className="key-btn" onClick={() => sendKey('\t')} title="Tab">Tab</button>
      <button
        className={`key-btn modifier${ctrlActive ? ' active' : ''}`}
        onClick={toggleCtrl}
        title="Ctrl (sticky)"
      >Ctrl</button>
      <button
        className={`key-btn modifier${altActive ? ' active' : ''}`}
        onClick={toggleAlt}
        title="Alt (sticky)"
      >Alt</button>
      <button className="key-btn" onClick={() => sendKey('|')} title="Pipe">|</button>
      <button className="key-btn" onClick={() => sendKey('~')} title="Tilde">~</button>
      <div className="arrow-group">
        <button className="key-btn arrow" onClick={() => sendKey('\x1b[A')} title="Up" aria-label="Arrow up">&#9650;</button>
        <button className="key-btn arrow" onClick={() => sendKey('\x1b[B')} title="Down" aria-label="Arrow down">&#9660;</button>
        <button className="key-btn arrow" onClick={() => sendKey('\x1b[D')} title="Left" aria-label="Arrow left">&#9664;</button>
        <button className="key-btn arrow" onClick={() => sendKey('\x1b[C')} title="Right" aria-label="Arrow right">&#9654;</button>
      </div>
    </div>
  );
}
