import { useEffect, useState } from 'react';
import { Route, Routes } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import Layout from './components/Layout';
import ConfigPage from './pages/Config';
import LibraryPage from './pages/SongLibrary';
import LogsPage from './pages/Logs';
import QueuePage from './pages/Queue';
import type { DanmakuEvent } from './types';

export default function App() {
  const [logs, setLogs] = useState<string[]>([]);
  const [recentDanmaku, setRecentDanmaku] = useState<DanmakuEvent[]>([]);

  useEffect(() => {
    const logPromise = listen<string>('log-event', (event) => {
      setLogs((current) => [`${new Date().toLocaleTimeString()} ${event.payload}`, ...current].slice(0, 500));
    });
    const danmakuPromise = listen<DanmakuEvent>('danmaku', (event) => {
      setRecentDanmaku((current) => [event.payload, ...current].slice(0, 12));
    });
    return () => {
      logPromise.then((unlisten) => unlisten()).catch(() => undefined);
      danmakuPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);

  return (
    <Layout>
      <Routes>
        <Route path="/" element={<QueuePage recentDanmaku={recentDanmaku} />} />
        <Route path="/library" element={<LibraryPage />} />
        <Route path="/config" element={<ConfigPage />} />
        <Route path="/logs" element={<LogsPage externalLogs={logs} />} />
      </Routes>
    </Layout>
  );
}
