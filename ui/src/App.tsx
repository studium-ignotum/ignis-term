import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { ConnectionProvider } from './lib/context/ConnectionContext';
import { TerminalProvider } from './lib/context/TerminalContext';
import { TabsProvider } from './lib/context/TabsContext';
import TerminalPage from './routes/TerminalPage';
import LoginPage from './routes/LoginPage';

export default function App() {
  return (
    <BrowserRouter>
      <ConnectionProvider>
        <TerminalProvider>
          <TabsProvider>
            <Routes>
              <Route path="/" element={<TerminalPage />} />
              <Route path="/login" element={<LoginPage />} />
              <Route path="*" element={<Navigate to="/login" replace />} />
            </Routes>
          </TabsProvider>
        </TerminalProvider>
      </ConnectionProvider>
    </BrowserRouter>
  );
}
