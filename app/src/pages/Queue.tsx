import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api } from '../lib/tauri';
import type { DanmakuEvent, SongRequest } from '../types';

export default function QueuePage() {
  const [queue, setQueue] = useState<SongRequest[]>([]);
  const [gameConnected, setGameConnected] = useState(false);
  const [danmaku, setDanmaku] = useState<DanmakuEvent[]>([]);
  const [message, setMessage] = useState('准备就绪');

  const refresh = useCallback(async () => {
    const [items, connected] = await Promise.all([api.getQueue(), api.getGameConnectionStatus()]);
    setQueue(items);
    setGameConnected(connected);
  }, []);

  useEffect(() => {
    refresh().catch((error) => setMessage(String(error)));
    const queuePromise = listen('queue-updated', () => refresh().catch((error) => setMessage(String(error))));
    const danmakuPromise = listen<DanmakuEvent>('danmaku', (event) => {
      setDanmaku((current) => [event.payload, ...current].slice(0, 12));
      if (event.payload.is_song_request) {
        refresh().catch((error) => setMessage(String(error)));
      }
    });
    return () => {
      queuePromise.then((unlisten) => unlisten()).catch(() => undefined);
      danmakuPromise.then((unlisten) => unlisten()).catch(() => undefined);
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

  async function handleRemove(songId: number) {
    await api.removeFromQueue(songId);
    await refresh();
  }

  async function handleReconnect() {
    const connected = await api.reconnectGame();
    setGameConnected(connected);
    setMessage(connected ? '已连接到游戏进程' : '未找到 DivaMegaMix.exe');
  }

  return (
    <section className="page queue-page">
      <header className="hero-panel">
        <div>
          <p className="eyebrow">Live Stage Control</p>
          <h1>点歌队列</h1>
          <p className="muted">弹幕点歌会自动入队；也可以在歌曲库里手动搜索并切歌。</p>
        </div>
        <div className="status-stack">
          <span className={`status-pill ${gameConnected ? 'ok' : 'bad'}`}>{gameConnected ? '游戏已连接' : '游戏未连接'}</span>
          <button type="button" className="secondary-button" onClick={handleReconnect}>重新连接游戏</button>
        </div>
      </header>

      <div className="grid-two">
        <div className="panel queue-panel">
          <div className="panel-header">
            <div>
              <h2>等待播放</h2>
              <p className="muted">当前 {queue.length} 首</p>
            </div>
            <button type="button" className="primary-button" onClick={handleNextSong}>切下一首</button>
          </div>
          {queue.length === 0 ? (
            <div className="empty-state">等待点歌中…</div>
          ) : (
            <ol className="song-list">
              {queue.map((item, index) => (
                <li key={`${item.song_id}-${item.timestamp}`} className="song-card">
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
            {danmaku.length === 0 ? <div className="empty-state small">弹幕连接后会显示在这里</div> : null}
            {danmaku.map((item) => (
              <div key={`${item.timestamp}-${item.content}`} className={`danmaku-item ${item.is_song_request ? 'highlight' : ''}`}>
                <span>{item.user_name}</span>
                <p>{item.content}</p>
              </div>
            ))}
          </div>
          <div className="message-bar">{message}</div>
        </div>
      </div>
    </section>
  );
}
