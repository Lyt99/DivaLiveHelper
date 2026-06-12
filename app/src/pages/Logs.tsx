import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api } from '../lib/tauri';

interface LogsPageProps {
  externalLogs: string[];
}

export default function LogsPage({ externalLogs }: LogsPageProps) {
  const [gameConnected, setGameConnected] = useState(false);
  const [danmakuConnected, setDanmakuConnected] = useState(false);
  const [danmakuConnecting, setDanmakuConnecting] = useState(false);
  const [message, setMessage] = useState('');

  async function refresh() {
    const [game, danmaku] = await Promise.all([api.getGameConnectionStatus(), api.getDanmakuStatus()]);
    setGameConnected(game);
    setDanmakuConnected(danmaku.connected);
  }

  useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
    const connectionPromise = listen<string>('connection-status', (event) => {
      setDanmakuConnecting(false);
      setMessage(event.payload);
      refresh().catch((error) => setMessage(String(error)));
    });
    return () => {
      connectionPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);

  async function connectDanmaku() {
    try {
      setDanmakuConnecting(true);
      const config = await api.getConfig();
      await api.startDanmaku(config.room_id);
      setMessage('正在连接弹幕服务器');
    } catch (error) {
      setDanmakuConnecting(false);
      setMessage(String(error));
    }
  }

  async function stopDanmaku() {
    try {
      setDanmakuConnecting(false);
      await api.stopDanmaku();
      await refresh();
      setMessage('已断开直播间弹幕');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function reconnectGame() {
    try {
      await api.reconnectGame();
      await refresh();
    } catch (error) {
      setMessage(String(error));
    }
  }

  return (
    <section className="page">
      <header className="page-header"><p className="eyebrow">Telemetry</p><h1>日志 / 状态</h1></header>
      <div className="status-grid">
        <StatusCard title="游戏进程" active={gameConnected} action="重新连接" onClick={reconnectGame} />
        <StatusCard title="B站弹幕" active={danmakuConnected} pending={danmakuConnecting} action={danmakuConnecting ? '连接中…' : danmakuConnected ? '断开' : '连接'} onClick={danmakuConnected ? stopDanmaku : connectDanmaku} />
      </div>
      <div className="panel log-panel">
        {externalLogs.length === 0 ? <div className="empty-state small">暂无日志</div> : null}
        {externalLogs.map((line) => <div key={line} className="log-line">{line}</div>)}
      </div>
      <div className="message-bar">{message}</div>
    </section>
  );
}

function StatusCard({ title, active, pending = false, action, onClick }: { title: string; active: boolean; pending?: boolean; action: string; onClick: () => void }) {
  return <div className="panel status-card"><span className={`status-pill ${active ? 'ok' : 'bad'}`}>{pending ? '连接中' : active ? '在线' : '离线'}</span><h2>{title}</h2><button className="secondary-button" type="button" onClick={onClick} disabled={pending}>{action}</button></div>;
}
