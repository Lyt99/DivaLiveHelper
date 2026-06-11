import { useEffect, useState } from 'react';
import { Route, Routes } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import Layout from './components/Layout';
import ConfigPage from './pages/Config';
import LibraryPage from './pages/SongLibrary';
import LogsPage from './pages/Logs';
import QueuePage from './pages/Queue';

export default function App() {
  const [logs, setLogs] = useState<string[]>([]);

  useEffect(() => {
    const unlistenPromise = listen<string>('log-event', (event) => {
      setLogs((current) => [`${new Date().toLocaleTimeString()} ${event.payload}`, ...current].slice(0, 500));
    });
    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);

  return (
    <Layout>
      <Routes>
        <Route path="/" element={<QueuePage />} />
        <Route path="/library" element={<LibraryPage />} />
        <Route path="/config" element={<ConfigPage />} />
        <Route path="/logs" element={<LogsPage externalLogs={logs} />} />
      </Routes>
    </Layout>
  );
}
