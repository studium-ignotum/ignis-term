import { useConnection, type ConnectionState } from '../context/ConnectionContext';
import './ConnectionStatus.css';

const stateDisplay: Record<ConnectionState, { label: string; color: string; icon: string }> = {
  disconnected: { label: 'Disconnected', color: 'text-gray-500', icon: '○' },
  connecting: { label: 'Connecting...', color: 'text-yellow-500', icon: '◐' },
  authenticating: { label: 'Authenticating...', color: 'text-yellow-500', icon: '◑' },
  connected: { label: 'Connected', color: 'text-green-500', icon: '●' },
  reconnecting: { label: 'Reconnecting...', color: 'text-orange-500', icon: '◐' },
};

export default function ConnectionStatus() {
  const { state } = useConnection();
  const display = stateDisplay[state];

  return (
    <div className="connection-status">
      <span className={`icon ${display.color}`}>{display.icon}</span>
      <span className={`label ${display.color}`}>{display.label}</span>
    </div>
  );
}
