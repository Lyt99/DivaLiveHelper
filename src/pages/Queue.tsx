import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, emptyConfig } from '../lib/tauri';
import { reportStatus, setConnectionState, useShellState } from '../lib/status';
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
  const { gameConnected } = useShellState();
  const [danmakuStatus, setDanmakuStatus] = useState<DanmakuStatus>({ connected: false, room_id: 0 });
  const [danmakuConnecting, setDanmakuConnecting] = useState(false);
  const [debugText, setDebugText] = useState('点歌 ');
  const [debugResult, setDebugResult] = useState<DebugSongRequestResult | null>(null);
  const [failures, setFailures] = useState<FailureToast[]>([]);

  const refresh = useCallback(async () => {
    const [items, status] = await Promise.all([api.getQueue(), api.getDanmakuStatus()]);
    setQueue(items);
    setDanmakuStatus(status);
    // 顺手同步到底部状态栏，不用重复请求
    setConnectionState({ danmakuConnected: status.connected, roomId: status.room_id });
  }, []);

  useEffect(() => {
    api.getConfig().then(setConfig).catch((error) => reportStatus(String(error)));
    refresh().catch((error) => reportStatus(String(error)));
    const queuePromise = listen('queue-updated', () => refresh().catch((error) => reportStatus(String(error))));
    const danmakuPromise = listen<DanmakuEvent>('danmaku', (event) => {
      if (event.payload.is_song_request) {
        refresh().catch((error) => reportStatus(String(error)));
      }
    });
    const connectionPromise = listen<string>('connection-status', () => {
      setDanmakuConnecting(false);
      refresh().catch((error) => reportStatus(String(error)));
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
      reportStatus(song ? `已切换：${song.song_name}` : '队列为空');
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function handleOpenOverlay() {
    try {
      await api.openQueueOverlay();
      reportStatus('已打开悬浮窗');
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function handleRemove(songId: number) {
    await api.removeFromQueue(songId);
    await refresh();
  }


  async function handleStartDanmaku() {
    try {
      setDanmakuConnecting(true);
      reportStatus(`正在连接直播间 ${config.room_id}…`);
      await api.startDanmaku(config.room_id);
      refresh().catch((error) => reportStatus(String(error)));
    } catch (error) {
      setDanmakuConnecting(false);
      reportStatus(String(error));
    }
  }

  async function handleStopDanmaku() {
    try {
      setDanmakuConnecting(false);
      await api.stopDanmaku();
      await refresh();
      reportStatus('已断开直播间弹幕');
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function handleDebugSongRequest() {
    try {
      const result = await api.debugSongRequest(debugText);
      setDebugResult(result);
      reportStatus(result.message);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function handleDebugEnqueueSong() {
    try {
      const result = await api.debugEnqueueSong(debugText);
      setDebugResult(result);
      await refresh();
      reportStatus(result.message);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  const [nextSong, ...restQueue] = queue;

  return (
    <section className="page queue-page">
      <header className="toolbar">
        <div className="console-group">
          <span className="console-key">直播间</span>
          <span className="console-value">{config.room_id || '未设置'}</span>
          <button type="button" className="secondary-button button-sm" onClick={handleStartDanmaku} disabled={!config.room_id || danmakuStatus.connected || danmakuConnecting}>
            {danmakuConnecting ? '连接中…' : '连接'}
          </button>
          <button type="button" className="ghost-button button-sm" onClick={handleStopDanmaku} disabled={!danmakuStatus.connected}>断开</button>
        </div>
        <i className="console-sep" />
        <div className="console-group">
          <span className="console-key">游戏进程</span>
          <span className="console-value">DivaMegaMix.exe</span>
          <span className={`signal ${gameConnected ? 'ok' : 'bad'}`} title="每 2 秒自动检测游戏进程，切歌时自动打开进程"><i />{gameConnected ? '运行中' : '等待启动'}</span>
        </div>
        <div className="toolbar-actions">
          <button type="button" className="ghost-button" onClick={handleOpenOverlay}>打开悬浮窗</button>
          <button type="button" className="primary-button" onClick={handleNextSong}>切下一首</button>
        </div>
      </header>

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

      <div className="workspace">
        <div className="queue-side">
          <div className="side-head">
            <h2>队列</h2>
            <span className="panel-count">{queue.length} 首</span>
          </div>
          {!nextSong ? (
            <div className="queue-scroll">
              <div className="empty-state">等待观众点歌…</div>
            </div>
          ) : (
            <>
              {/* NEXT 焦点区固定在滚动区上方，始终可见 */}
              <div className="next-up">
                <div className="next-up-text">
                  <span className="next-up-label">NEXT</span>
                  <strong>{nextSong.song_name}</strong>
                  <span className="track-meta">#{nextSong.song_id} · {nextSong.requester}{nextSong.difficulty ? ` · ${nextSong.difficulty}★` : ''}</span>
                </div>
                <button type="button" className="ghost-button button-sm" onClick={() => handleRemove(nextSong.song_id)}>移除</button>
              </div>
              {restQueue.length > 0 ? (
                <div className="queue-scroll">
                  <ol className="tracklist">
                    {restQueue.map((item, index) => (
                      <li key={`${item.song_id}-${item.timestamp}`} className="track">
                        <span className="track-index">{String(index + 2).padStart(2, '0')}</span>
                        <div className="track-main">
                          <strong>{item.song_name}</strong>
                          <span className="track-meta">#{item.song_id} · {item.requester}{item.difficulty ? ` · ${item.difficulty}★` : ''}</span>
                        </div>
                        <button type="button" className="ghost-button button-sm track-remove" onClick={() => handleRemove(item.song_id)}>移除</button>
                      </li>
                    ))}
                  </ol>
                </div>
              ) : null}
            </>
          )}
        </div>

        <div className="feed-side">
          <div className="side-head">
            <h2>实时弹幕</h2>
            <span className="panel-count">最近 {recentDanmaku.length} 条</span>
          </div>
          <div className="feed" aria-live="polite">
            {recentDanmaku.length === 0 ? <div className="empty-state small">连接直播间后显示在这里</div> : null}
            {recentDanmaku.map((item) => (
              <div
                key={`${item.timestamp}-${item.content}`}
                className={`feed-item ${item.is_song_request ? 'request' : ''}`}
                aria-label={`${item.user_name}${item.is_song_request ? ' 点歌' : ''}：${item.content}`}
              >
                <span className="feed-user">{item.user_name}</span>
                <p>{item.content}</p>
              </div>
            ))}
          </div>
        </div>
      </div>

      <details className="debug-collapsible">
        <summary>
          离线调试
          <span className="debug-summary-note">不连直播间也能测点歌流程</span>
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
            <button type="button" className="secondary-button button-sm" onClick={handleDebugSongRequest}>前缀测试</button>
            <button type="button" className="primary-button button-sm" onClick={handleDebugEnqueueSong} disabled={!debugText.trim()}>按弹幕处理并加入</button>
          </div>
        </div>
        {debugResult ? (
          <div className={`debug-result ${debugResult.matched ? 'ok' : 'bad'}`}>
            <span>{debugResult.added ? '已加入' : debugResult.matched ? '可用' : debugResult.is_song_request ? '未命中' : '非点歌'}</span>
            <strong>{debugResult.song_name ?? (debugResult.query || debugText)}</strong>
            <p>{debugResult.message}{debugResult.requester ? ` · 点歌人：${debugResult.requester}` : ''}</p>
            <p className="debug-hint">「按弹幕处理并加入」会走真实直播间弹幕流程：前缀优先，非前缀时按当前 LLM 设置识别。</p>
          </div>
        ) : null}
      </details>
    </section>
  );
}
