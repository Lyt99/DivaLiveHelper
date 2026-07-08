import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, emptyConfig } from '../lib/tauri';
import type { AppConfig, DanmakuEvent, DanmakuStatus, DebugSongRequestResult, SongRequest, SongRequestFailure } from '../types';

interface FailureToast extends SongRequestFailure {
  id: number;
}

interface QueuePageProps {
  recentDanmaku: DanmakuEvent[];
}

export default function QueuePage({ recentDanmaku }: QueuePageProps) {
  const [queue, setQueue] = useState<SongRequest[]>([]);
  const [config, setConfig] = useState<AppConfig>(emptyConfig);
  const [gameConnected, setGameConnected] = useState(false);
  const [danmakuStatus, setDanmakuStatus] = useState<DanmakuStatus>({ connected: false, room_id: 0 });
  const [danmakuConnecting, setDanmakuConnecting] = useState(false);
  const [debugText, setDebugText] = useState('点歌 ');
  const [debugResult, setDebugResult] = useState<DebugSongRequestResult | null>(null);
  const [message, setMessage] = useState('准备就绪');
  const [failures, setFailures] = useState<FailureToast[]>([]);

  const refresh = useCallback(async () => {
    const [items, connected, status] = await Promise.all([api.getQueue(), api.getGameConnectionStatus(), api.getDanmakuStatus()]);
    setQueue(items);
    setGameConnected(connected);
    setDanmakuStatus(status);
  }, []);

  useEffect(() => {
    api.getConfig().then(setConfig).catch((error) => setMessage(String(error)));
    refresh().catch((error) => setMessage(String(error)));
    const queuePromise = listen('queue-updated', () => refresh().catch((error) => setMessage(String(error))));
    const danmakuPromise = listen<DanmakuEvent>('danmaku', (event) => {
      if (event.payload.is_song_request) {
        refresh().catch((error) => setMessage(String(error)));
      }
    });
    const connectionPromise = listen<string>('connection-status', (event) => {
      setDanmakuConnecting(false);
      setMessage(event.payload);
      refresh().catch((error) => setMessage(String(error)));
    });
    const failurePromise = listen<SongRequestFailure>('song-request-failed', (event) => {
      const toast: FailureToast = { id: Date.now() + Math.random(), ...event.payload };
      setFailures((prev) => [...prev.slice(-2), toast]);
      window.setTimeout(() => {
        setFailures((prev) => prev.filter((item) => item.id !== toast.id));
      }, 4000);
    });
    return () => {
      queuePromise.then((unlisten) => unlisten()).catch(() => undefined);
      danmakuPromise.then((unlisten) => unlisten()).catch(() => undefined);
      connectionPromise.then((unlisten) => unlisten()).catch(() => undefined);
      failurePromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, [refresh]);

  async function handleNextSong() {
    try {
      const song = await api.nextSong();
      await refresh();
      setMessage(song ? `已切换：${song.song_name}` : '队列为空');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function handleOpenOverlay() {
    try {
      await api.openQueueOverlay();
      setMessage('已打开悬浮窗');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function handleRemove(songId: number) {
    await api.removeFromQueue(songId);
    await refresh();
  }

  async function handleReconnect() {
    try {
      const connected = await api.reconnectGame();
      setGameConnected(connected);
      setMessage(connected ? '已连接到游戏进程' : '未找到 DivaMegaMix.exe');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function handleStartDanmaku() {
    try {
      setDanmakuConnecting(true);
      setMessage(`正在连接直播间 ${config.room_id}…`);
      await api.startDanmaku(config.room_id);
      refresh().catch((error) => setMessage(String(error)));
    } catch (error) {
      setDanmakuConnecting(false);
      setMessage(String(error));
    }
  }

  async function handleStopDanmaku() {
    try {
      setDanmakuConnecting(false);
      await api.stopDanmaku();
      await refresh();
      setMessage('已断开直播间弹幕');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function handleDebugSongRequest() {
    try {
      const result = await api.debugSongRequest(debugText);
      setDebugResult(result);
      setMessage(result.message);
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function handleDebugEnqueueSong() {
    try {
      const result = await api.debugEnqueueSong(debugText);
      setDebugResult(result);
      await refresh();
      setMessage(result.message);
    } catch (error) {
      setMessage(String(error));
    }
  }

  return (
    <section className="page queue-page">
      <header className="hero-panel">
        <div>
          <p className="eyebrow">Live Stage</p>
          <h1>点歌</h1>
          <p className="muted">先连接直播间和游戏，再让弹幕点歌自动入队</p>
        </div>
        <div className="hero-side">
          <div className="status-stack">
            {danmakuStatus.room_id ? (
              <div className="room-id-display" aria-label={`直播间房间号 ${danmakuStatus.room_id}`}>
                <span className="room-id-label">ROOM</span>
                <span className="room-id-value">#{danmakuStatus.room_id}</span>
              </div>
            ) : null}
            <span className={`status-pill ${danmakuStatus.connected ? 'ok' : 'bad'}`}>{danmakuStatus.connected ? `直播间 ${danmakuStatus.room_id} 已连接` : '直播间未连接'}</span>
            <span className={`status-pill ${gameConnected ? 'ok' : 'bad'}`}>{gameConnected ? '游戏已连接' : '游戏未连接'}</span>
          </div>
          <div className="hero-actions">
            <button type="button" className="primary-button" onClick={handleNextSong}>切下一首</button>
            <button type="button" className="secondary-button" onClick={handleOpenOverlay}>打开悬浮窗</button>
          </div>
        </div>
      </header>

      <div className="control-grid">
        <div className="panel connection-card">
          <div>
            <h2>连接直播间</h2>
            <p className="muted">当前房间号：{config.room_id || '未设置'}</p>
          </div>
          <div className="inline-actions">
            <button type="button" className="primary-button" onClick={handleStartDanmaku} disabled={!config.room_id || danmakuStatus.connected || danmakuConnecting}>{danmakuConnecting ? '连接中…' : '连接直播间'}</button>
            <button type="button" className="ghost-button" onClick={handleStopDanmaku} disabled={!danmakuStatus.connected}>断开</button>
          </div>
        </div>
        <div className="panel connection-card">
          <div>
            <h2>连接游戏</h2>
            <p className="muted">目标进程：DivaMegaMix.exe</p>
          </div>
          <div className="inline-actions">
            <button type="button" className="primary-button" onClick={handleReconnect}>{gameConnected ? '重新连接游戏' : '连接游戏'}</button>
          </div>
        </div>
      </div>

      {failures.length > 0 ? (
        <div className="failure-toasts" aria-live="assertive">
          {failures.map((item) => (
            <div key={item.id} className="failure-toast" role="alert">
              <span className="failure-toast-requester">{item.requester}</span>
              <span className="failure-toast-message">{item.message}</span>
            </div>
          ))}
        </div>
      ) : null}

      <div className="grid-two">
        <div className="panel queue-panel">
          <div className="panel-header">
            <div>
              <h2>
                等待播放
                <span className="queue-count-badge" aria-label={`${queue.length} 首待播`}>{queue.length}</span>
              </h2>
              <p className="muted">当前 {queue.length} 首</p>
            </div>
          </div>
          {queue.length === 0 ? (
            <div className="empty-state">等待观众点歌中…</div>
          ) : (
            <ol className="song-list">
              {queue.map((item, index) => (
                <li
                  key={`${item.song_id}-${item.timestamp}`}
                  className="song-card"
                  data-pos={index === 0 ? 'first' : undefined}
                >
                  <span className="song-index">{String(index + 1).padStart(2, '0')}</span>
                  <div className="song-main">
                    <strong>{item.song_name}</strong>
                    <span>#{item.song_id} · {item.requester}{item.difficulty ? ` · ${item.difficulty}★` : ''}</span>
                  </div>
                  <button type="button" className="ghost-button" onClick={() => handleRemove(item.song_id)}>移除</button>
                </li>
              ))}
            </ol>
          )}
        </div>

        <div className="panel danmaku-panel">
          <div className="panel-header compact">
            <h2>实时弹幕</h2>
            <span className="muted">最近 12 条</span>
          </div>
          <div className="danmaku-list">
            {recentDanmaku.length === 0 ? <div className="empty-state small">弹幕连接后会显示在这里</div> : null}
            {recentDanmaku.map((item) => (
              <div
                key={`${item.timestamp}-${item.content}`}
                className={`danmaku-item ${item.is_song_request ? 'highlight' : ''}`}
                aria-label={`${item.user_name}${item.is_song_request ? ' 点歌' : ''}：${item.content}`}
              >
                <span>{item.user_name}</span>
                <p>{item.content}</p>
              </div>
            ))}
          </div>
          <div className="message-bar" aria-live="polite">{message}</div>
        </div>
      </div>

      <details className="debug-collapsible">
        <summary>
          <span className="eyebrow debug-summary-eyebrow">离线调试</span>
          <span className="muted">点歌调试</span>
        </summary>
        <div className="debug-command-row">
          <input
            value={debugText}
            onChange={(event) => {
              setDebugText(event.target.value);
              setDebugResult(null);
            }}
            placeholder="例如：点歌 世界第一公主殿下"
          />
          <div className="inline-actions">
            <button type="button" className="secondary-button" onClick={handleDebugSongRequest}>前缀测试</button>
            <button type="button" className="primary-button" onClick={handleDebugEnqueueSong} disabled={!debugText.trim()}>按弹幕处理并加入</button>
          </div>
        </div>
        {debugResult ? (
          <div className={`debug-result ${debugResult.matched ? 'ok' : 'bad'}`}>
            <span>{debugResult.added ? '已加入' : debugResult.matched ? '可用' : debugResult.is_song_request ? '未命中' : '非点歌'}</span>
            <strong>{debugResult.song_name ?? (debugResult.query || debugText)}</strong>
            <p>{debugResult.message}{debugResult.requester ? ` · 点歌人：${debugResult.requester}` : ''}</p>
            <p className="debug-hint">“按弹幕处理并加入”会走真实直播间弹幕流程：前缀优先，非前缀时按当前 LLM 设置识别。</p>
          </div>
        ) : null}
      </details>
    </section>
  );
}
