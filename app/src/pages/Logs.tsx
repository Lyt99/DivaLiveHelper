import { useEffect, useState } from 'react';
import { api } from '../lib/tauri';

interface LogsPageProps {
  externalLogs: string[];
}

export default function LogsPage({ externalLogs }: LogsPageProps) {
  const [gameConnected, setGameConnected] = useState(false);
  const [danmakuConnected, setDanmakuConnected] = useState(false);
  const [message, setMessage] = useState('');

  async function refresh() {
    const [game, danmaku] = await Promise.all([api.getGameConnectionStatus(), api.getDanmakuStatus()]);
    setGameConnected(game);
    setDanmakuConnected(danmaku.connected);
  }

  useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
  }, []);

  async function connectDanmaku() {
    try {
      const config = await api.getConfig();
      await api.startDanmaku(config.room_id);
      await refresh();
      setMessage('正在连接弹幕服务器');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function stopDanmaku() {
    await api.stopDanmaku();
    await refresh();
  }

  return (
    <section className="page">
      <header className="page-header"><p className="eyebrow">Telemetry</p><h1>日志 / 状态</h1></header>
      <div className="status-grid">
        <StatusCard title="游戏进程" active={gameConnected} action="重新连接" onClick={() => api.reconnectGame().then(refresh)} />
        <StatusCard title="B站弹幕" active={danmakuConnected} action={danmakuConnected ? '断开' : '连接'} onClick={danmakuConnected ? stopDanmaku : connectDanmaku} />
      </div>
      <div className="panel log-panel">
        {externalLogs.length === 0 ? <div className="empty-state small">暂无日志</div> : null}
        {externalLogs.map((line) => <div key={line} className="log-line">{line}</div>)}
      </div>
      <div className="message-bar">{message}</div>
    </section>
  );
}

function StatusCard({ title, active, action, onClick }: { title: string; active: boolean; action: string; onClick: () => void }) {
  return <div className="panel status-card"><span className={`status-pill ${active ? 'ok' : 'bad'}`}>{active ? '在线' : '离线'}</span><h2>{title}</h2><button className="secondary-button" type="button" onClick={onClick}>{action}</button></div>;
}
