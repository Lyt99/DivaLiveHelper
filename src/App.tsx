import { useEffect, useState } from 'react';
import { Route, Routes, useLocation } from 'react-router-dom';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import Layout from './components/Layout';
import ConfigPage from './pages/Config';
import LibraryPage from './pages/SongLibrary';
import LogsPage from './pages/Logs';
import OverlayPage from './pages/Overlay';
import QueuePage from './pages/Queue';
import WizardPage from './pages/Wizard';
import { api } from './lib/tauri';
import type { DanmakuEvent } from './types';

export default function App() {
  const [logs, setLogs] = useState<string[]>([]);
  const [recentDanmaku, setRecentDanmaku] = useState<DanmakuEvent[]>([]);
  const [firstRun, setFirstRun] = useState<boolean | null>(null);
  const location = useLocation();
  const [isOverlayWindowLabel] = useState(isOverlayWindow);
  const isOverlayRoute = location.pathname === '/overlay' || isOverlayWindowLabel;

  useEffect(() => {
    if (isOverlayRoute) return;

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
  }, [isOverlayRoute]);

  useEffect(() => {
    if (isOverlayRoute) return;

    api.isFirstRun().then(setFirstRun).catch(() => setFirstRun(false));
  }, [isOverlayRoute]);

  // 悬浮窗是独立轻量入口：不要等待主窗口 first-run 检查，避免 invoke 初始化失败时白屏。
  if (isOverlayRoute) {
    return <OverlayPage />;
  }

  if (firstRun === null) return null;

  if (firstRun) return <WizardPage />;

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

function isOverlayWindow() {
  try {
    return getCurrentWindow().label === 'overlay';
  } catch {
    return false;
  }
}
