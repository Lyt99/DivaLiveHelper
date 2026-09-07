import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api } from '../lib/tauri';
import { refreshDanmakuState, reportStatus, useShellState } from '../lib/status';

interface LogsPageProps {
  externalLogs: string[];
}

export default function LogsPage({ externalLogs }: LogsPageProps) {
  const { gameConnected, danmakuConnected } = useShellState();
  const [danmakuConnecting, setDanmakuConnecting] = useState(false);

  useEffect(() => {
    refreshDanmakuState().catch((error) => reportStatus(String(error)));
    const connectionPromise = listen<string>('connection-status', () => {
      setDanmakuConnecting(false);
      refreshDanmakuState().catch((error) => reportStatus(String(error)));
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
      reportStatus('正在连接弹幕服务器');
    } catch (error) {
      setDanmakuConnecting(false);
      reportStatus(String(error));
    }
  }

  async function stopDanmaku() {
    try {
      setDanmakuConnecting(false);
      await api.stopDanmaku();
      await refreshDanmakuState();
      reportStatus('已断开直播间弹幕');
    } catch (error) {
      reportStatus(String(error));
    }
  }


  return (
    <section className="page logs-page">
      <header className="toolbar">
        <div className="console-group">
          <span className={`signal ${gameConnected ? 'ok' : 'bad'}`}><i />{gameConnected ? '运行中' : '未启动'}</span>
          <span className="console-key">游戏进程</span>
          <span className="hint">每 2 秒自动检测</span>
        </div>
        <i className="console-sep" />
        <div className="console-group">
          <span className={`signal ${danmakuConnecting ? 'busy' : danmakuConnected ? 'ok' : 'bad'}`}>
            <i />{danmakuConnecting ? '连接中' : danmakuConnected ? '在线' : '离线'}
          </span>
          <span className="console-key">B 站弹幕</span>
          <button type="button" className="secondary-button button-sm" onClick={danmakuConnected ? stopDanmaku : connectDanmaku} disabled={danmakuConnecting}>
            {danmakuConnecting ? '连接中…' : danmakuConnected ? '断开' : '连接'}
          </button>
        </div>
      </header>
      <div className="log-stream">
        {externalLogs.length === 0 ? <div className="empty-state small">暂无日志</div> : null}
        {externalLogs.map((line) => <LogLine key={line} line={line} />)}
      </div>
    </section>
  );
}

/* 日志格式为 "时间 内容"，把时间戳拆出来弱化显示 */
function LogLine({ line }: { line: string }) {
  const splitAt = line.indexOf(' ');
  if (splitAt <= 0) return <div className="log-line">{line}</div>;
  return (
    <div className="log-line">
      <span className="log-time">{line.slice(0, splitAt)}</span>
      {line.slice(splitAt + 1)}
    </div>
  );
}
