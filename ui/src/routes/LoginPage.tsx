import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConnection } from '../lib/context/ConnectionContext';
import './LoginPage.css';

export default function LoginPage() {
  const [sessionCode, setSessionCode] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const navigate = useNavigate();
  const { state, error, isConnected, connect } = useConnection();

  // Redirect to terminal if already connected
  useEffect(() => {
    if (isConnected) {
      navigate('/');
    }
  }, [isConnected, navigate]);

  // Reset submitting state on error
  useEffect(() => {
    if (error) {
      setIsSubmitting(false);
    }
  }, [error]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();

    const code = sessionCode.toUpperCase().replace(/\s/g, '');
    if (code.length !== 6) return;

    setIsSubmitting(true);
    connect(code, () => {
      navigate('/');
    });
  }

  // Show reconnecting spinner while auto-rejoin is in progress
  if (state === 'rejoining') {
    return (
      <div className="login-container">
        <div className="login-box">
          <h1>Reconnecting...</h1>
          <p className="subtitle">Restoring your session</p>
          <div className="reconnecting-spinner" />
        </div>
      </div>
    );
  }

  return (
    <div className="login-container">
      <div className="login-box">
        <h1>Connect to Terminal</h1>
        <p className="subtitle">Enter the session code shown on your Mac</p>

        <form onSubmit={handleSubmit}>
          <div className="input-wrapper">
            <label htmlFor="code" className="sr-only">Session Code</label>
            <input
              id="code"
              type="text"
              value={sessionCode}
              onChange={(e) => setSessionCode(e.target.value)}
              placeholder="ABC123"
              maxLength={6}
              autoComplete="off"
              autoCapitalize="characters"
              spellCheck={false}
              className="code-input"
              disabled={isSubmitting}
            />
          </div>

          {error && (
            <div className="error-box">
              {error}
            </div>
          )}

          <button
            type="submit"
            className="btn-primary"
            disabled={sessionCode.length !== 6 || isSubmitting}
          >
            {state === 'connecting' || state === 'authenticating'
              ? 'Connecting...'
              : 'Connect'}
          </button>
        </form>
      </div>
    </div>
  );
}
