import { useCallback, useEffect, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api } from '../lib/tauri';
import type { SongRequest, SongRequestFailure } from '../types';

interface FailureToast extends SongRequestFailure {
  id: number;
}

/**
 * 悬浮点歌队列窗口
 *
 * - 监听 `queue-updated` 事件 + 每 2s 轮询，双保险
 * - 自带可拖拽标题栏（pin 置顶切换 + 关闭）
 * - 样式与 OBS 覆盖层一致，全部使用 `ov-` 前缀类名避免污染全局
 */
export default function Overlay() {
  const [queue, setQueue] = useState<SongRequest[]>([]);
  const [pinned, setPinned] = useState(true);
  const [failures, setFailures] = useState<FailureToast[]>([]);

  const refresh = useCallback(async () => {
    try {
      const items = await api.getQueue();
      setQueue(items);
    } catch {
      // 静默失败 —— 悬浮窗保持最后一次已知状态
    }
  }, []);

  useEffect(() => {
    void refresh();
    let cancelled = false;
    let unlistenFn: UnlistenFn | undefined;
    let unlistenFailuresFn: UnlistenFn | undefined;

    void listen('queue-updated', () => {
      void refresh();
    })
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
        } else {
          unlistenFn = unlisten;
        }
      })
      .catch(() => undefined);

    void listen<SongRequestFailure>('song-request-failed', (event) => {
      const toast: FailureToast = { id: Date.now() + Math.random(), ...event.payload };
      setFailures((prev) => [...prev.slice(-2), toast]); // 最多保留 3 条
      window.setTimeout(() => {
        setFailures((prev) => prev.filter((item) => item.id !== toast.id));
      }, 4000);
    })
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
        } else {
          unlistenFailuresFn = unlisten;
        }
      })
      .catch(() => undefined);

    const intervalId = window.setInterval(() => {
      void refresh();
    }, 2000);

    return () => {
      cancelled = true;
      if (unlistenFn) unlistenFn();
      if (unlistenFailuresFn) unlistenFailuresFn();
      window.clearInterval(intervalId);
    };
  }, [refresh]);

  const handleTitlebarMouseDown = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    void getCurrentWindow()
      .startDragging()
      .catch(() => undefined);
  };

  const stopDrag = (event: ReactMouseEvent<HTMLElement>) => {
    // 标题栏上的按钮不触发拖拽
    event.stopPropagation();
  };

  const handleClose = () => {
    void getCurrentWindow()
      .close()
      .catch(() => undefined);
  };

  const handleTogglePin = () => {
    const next = !pinned;
    setPinned(next);
    void getCurrentWindow()
      .setAlwaysOnTop(next)
      .catch(() => undefined);
  };

  return (
    <div className="ov-root">
      <style>{STYLES}</style>
      <main className="ov-overlay" aria-label="点歌队列悬浮窗">
        <div
          className="ov-titlebar"
          onMouseDown={handleTitlebarMouseDown}
          role="toolbar"
          aria-label="悬浮窗标题栏"
        >
          <span className="ov-titlebar-title">点歌队列</span>
          <span className="ov-titlebar-count">
            <strong>{queue.length}</strong> 首等待
          </span>
          <div className="ov-titlebar-actions">
            <button
              type="button"
              className={`ov-titlebar-btn ov-pin-btn${pinned ? ' active' : ''}`}
              onClick={handleTogglePin}
              onMouseDown={stopDrag}
              title={pinned ? '取消置顶' : '置顶'}
              aria-pressed={pinned}
            >
              {pinned ? '置顶中' : '置顶'}
            </button>
            <button
              type="button"
              className="ov-titlebar-btn ov-close-btn"
              onClick={handleClose}
              onMouseDown={stopDrag}
              title="关闭"
              aria-label="关闭"
            >
              ×
            </button>
          </div>
        </div>

        <section className="ov-list" aria-live="polite">
          {queue.length === 0 ? (
            <div className="ov-empty">等待点歌…</div>
          ) : (
            queue.map((item, index) => (
              <div
                key={`${item.song_id}-${item.timestamp}`}
                className={`ov-song${index === 0 ? ' playing' : ''}`}
                style={{ animationDelay: `${index * 30}ms` }}
              >
                {item.difficulty != null ? (
                  <span className="ov-stars">{item.difficulty}★</span>
                ) : null}
                <div className="ov-song-main">
                  <span className="ov-name">{item.song_name}</span>
                  <span className="ov-requester">{item.requester}</span>
                </div>
              </div>
            ))
          )}
        </section>

        {failures.length > 0 ? (
          <section className="ov-failures" aria-live="assertive">
            {failures.map((item) => (
              <div key={item.id} className="ov-failure-toast" role="alert">
                <span className="ov-failure-requester">{item.requester}</span>
                <span className="ov-failure-message">{item.message}</span>
              </div>
            ))}
          </section>
        ) : null}
      </main>
    </div>
  );
}

const STYLES = `
html,
body,
#root {
  width: 100%;
  height: 100%;
  min-width: 0 !important;
  min-height: 0 !important;
  margin: 0;
  overflow: hidden;
  background: transparent !important;
}

.ov-root {
  background: transparent;
  color: #ffffff;
  font-family: "Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI", sans-serif;
  min-height: 100vh;
}
.ov-root *,
.ov-root *::before,
.ov-root *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

.ov-overlay {
  width: min(520px, calc(100vw - 24px));
  margin: 12px;
  border-radius: 12px;
  background: rgba(0, 0, 0, 0.55);
  border: 1px solid rgba(255, 255, 255, 0.08);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  color: #ffffff;
  overflow: hidden;
  box-shadow: 0 8px 28px rgba(0, 0, 0, 0.35);
  display: flex;
  flex-direction: column;
}

.ov-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  height: 32px;
  padding: 0 8px 0 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(0, 0, 0, 0.25);
  cursor: grab;
  user-select: none;
  -webkit-user-select: none;
}
.ov-titlebar:active { cursor: grabbing; }

.ov-titlebar-title {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.04em;
  color: rgba(255, 255, 255, 0.88);
  white-space: nowrap;
}

.ov-titlebar-count {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  white-space: nowrap;
  flex: 1 1 auto;
  text-align: right;
  padding-right: 4px;
}
.ov-titlebar-count strong {
  color: #ffffff;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  margin-right: 2px;
}

.ov-titlebar-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.ov-titlebar-btn {
  appearance: none;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.7);
  height: 22px;
  padding: 0 10px;
  border-radius: 6px;
  font-family: inherit;
  font-size: 12px;
  font-weight: 500;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: background 140ms ease, border-color 140ms ease, color 140ms ease;
}
.ov-titlebar-btn:hover {
  background: rgba(255, 255, 255, 0.10);
  color: #ffffff;
  border-color: rgba(255, 255, 255, 0.20);
}
.ov-titlebar-btn:focus-visible {
  outline: none;
  border-color: rgba(255, 255, 255, 0.45);
  box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.12);
}
.ov-pin-btn.active {
  background: rgba(255, 255, 255, 0.16);
  border-color: rgba(255, 255, 255, 0.32);
  color: #ffffff;
}
.ov-close-btn {
  width: 22px;
  padding: 0;
  font-size: 16px;
  line-height: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.ov-close-btn:hover {
  background: rgba(248, 81, 73, 0.22);
  border-color: rgba(248, 81, 73, 0.5);
  color: #ffffff;
}

.ov-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 18px 16px;
  max-height: calc(100vh - 56px);
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(255, 255, 255, 0.25) transparent;
}
.ov-list::-webkit-scrollbar { width: 6px; }
.ov-list::-webkit-scrollbar-track { background: transparent; }
.ov-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.25);
  border-radius: 3px;
}

.ov-song {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 8px;
  animation: ov-fade-in 280ms ease both;
}
.ov-song.playing {
  background: rgba(255, 255, 255, 0.07);
}

.ov-stars {
  font-size: 14px;
  color: rgba(255, 255, 255, 0.5);
  white-space: nowrap;
  flex-shrink: 0;
  min-width: 36px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.ov-song.playing .ov-stars { color: #e0e0e0; }

.ov-song-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1 1 auto;
}

.ov-name {
  font-size: 18px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: #ffffff;
}

.ov-requester {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ov-empty {
  padding: 24px 16px;
  border-radius: 8px;
  border: 1px dashed rgba(255, 255, 255, 0.08);
  text-align: center;
  color: rgba(255, 255, 255, 0.5);
  font-size: 14px;
}

@keyframes ov-fade-in {
  from { opacity: 0; }
  to   { opacity: 1; }
}

.ov-failures {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 0 18px 12px;
  flex-shrink: 0;
}

.ov-failure-toast {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  background: rgba(248, 81, 73, 0.18);
  border: 1px solid rgba(248, 81, 73, 0.35);
  animation: ov-fade-in 200ms ease both;
  overflow: hidden;
}

.ov-failure-requester {
  font-size: 12px;
  font-weight: 600;
  color: rgba(255, 200, 200, 0.95);
  white-space: nowrap;
  flex-shrink: 0;
  max-width: 80px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ov-failure-message {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.75);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
  flex: 1 1 auto;
}
`;
