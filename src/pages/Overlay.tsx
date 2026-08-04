import { useCallback, useEffect, useState } from 'react';
import type { MouseEvent as ReactMouseEvent } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api } from '../lib/tauri';
import type { SongRequest, SongRequestFailure } from '../types';

interface FailureToast extends SongRequestFailure {
  id: number;
}

/* 合法难度档位，用于星级着色（防止异常值混进 class） */
const DIFFICULTY_TIERS = new Set(['easy', 'normal', 'hard', 'extreme', 'exextreme']);

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

  const handleTogglePin = async () => {
    try {
      const next = await api.toggleQueueOverlayTop();
      setPinned(next);
    } catch {
      // 静默失败
    }
  };

  const startResize = (direction: string) => (event: ReactMouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    void getCurrentWindow()
      .startResizeDragging(direction as never)
      .catch(() => undefined);
  };

  return (
    <div className="ov-root">
      <style>{STYLES}</style>
      {/* 缩放手柄：无装饰窗口需要手动实现 */}
      <div className="ov-resize ov-resize-n" onMouseDown={startResize('North')} />
      <div className="ov-resize ov-resize-s" onMouseDown={startResize('South')} />
      <div className="ov-resize ov-resize-e" onMouseDown={startResize('East')} />
      <div className="ov-resize ov-resize-w" onMouseDown={startResize('West')} />
      <div className="ov-resize ov-resize-ne" onMouseDown={startResize('NorthEast')} />
      <div className="ov-resize ov-resize-nw" onMouseDown={startResize('NorthWest')} />
      <div className="ov-resize ov-resize-se" onMouseDown={startResize('SouthEast')} />
      <div className="ov-resize ov-resize-sw" onMouseDown={startResize('SouthWest')} />
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
              aria-label={pinned ? '取消置顶' : '置顶'}
              aria-pressed={pinned}
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                {pinned ? (
                  <>
                    <path d="M12 17v5" />
                    <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" />
                  </>
                ) : (
                  <>
                    <path d="M12 17v5" />
                    <path d="M15.93 9.34a2 2 0 0 1-.07 1.94l-1.7 2.83A2 2 0 0 0 14 15.5V16a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1v-.5a2 2 0 0 0-.3-1.06L8 11.28a2 2 0 0 1-.07-1.94L9.5 5.5h5z" />
                    <path d="M8.5 5.5 8 3l8 .5-.5 2" />
                  </>
                )}
              </svg>
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
            queue.map((item, index) => {
              const tier = DIFFICULTY_TIERS.has(item.difficulty_tier) ? ` tier-${item.difficulty_tier}` : '';
              return (
                <div
                  key={`${item.song_id}-${item.timestamp}`}
                  className={`ov-song${index === 0 ? ' playing' : ''}`}
                  style={{ animationDelay: `${index * 30}ms` }}
                >
                  <span className={`ov-pos${index === 0 ? ' ov-next' : ''}`}>
                    {index === 0 ? 'NEXT' : String(index + 1).padStart(2, '0')}
                  </span>
                  <span className="ov-name">{item.song_name}</span>
                  {item.difficulty != null ? (
                    <span className={`ov-stars${tier}`}>{item.difficulty.toFixed(1)}★</span>
                  ) : null}
                </div>
              );
            })
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
  font-family: "Segoe UI", "Microsoft YaHei UI", "Microsoft YaHei", "Yu Gothic UI", sans-serif;
  width: 100%;
  height: 100%;
}
.ov-root *,
.ov-root *::before,
.ov-root *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

/* 缩放手柄 —— 无装饰窗口需要自己实现 */
.ov-resize { position: fixed; z-index: 10; }
.ov-resize-n  { top: 0; left: 6px; right: 6px; height: 6px; cursor: ns-resize; }
.ov-resize-s  { bottom: 0; left: 6px; right: 6px; height: 6px; cursor: ns-resize; }
.ov-resize-e  { right: 0; top: 6px; bottom: 6px; width: 6px; cursor: ew-resize; }
.ov-resize-w  { left: 0; top: 6px; bottom: 6px; width: 6px; cursor: ew-resize; }
.ov-resize-ne { top: 0; right: 0; width: 12px; height: 12px; cursor: nesw-resize; }
.ov-resize-nw { top: 0; left: 0; width: 12px; height: 12px; cursor: nwse-resize; }
.ov-resize-se { bottom: 0; right: 0; width: 12px; height: 12px; cursor: nwse-resize; }
.ov-resize-sw { bottom: 0; left: 0; width: 12px; height: 12px; cursor: nesw-resize; }

.ov-overlay {
  width: 100%;
  height: 100%;
  margin: 0;
  border-radius: 5px;
  background: rgba(0, 0, 0, 0.55);
  border: none;
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
  color: #39c5bb;
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
.ov-pin-btn {
  width: 22px;
  height: 22px;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
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
  background: rgba(225, 40, 133, 0.25);
  border-color: rgba(225, 40, 133, 0.55);
  color: #ffffff;
}

.ov-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 5px 16px;
  flex: 1 1 0;
  min-height: 0;
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
  display: grid;
  grid-template-columns: 36px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  padding: 7px 12px;
  border-radius: 6px;
  animation: ov-fade-in 280ms ease both;
}

/* 队首 = 正在播放：Miku 青信号条 */
.ov-song.playing {
  background: rgba(57, 197, 187, 0.13);
  box-shadow: inset 2px 0 0 #39c5bb;
}

/* 序号 / NEXT 标签 */
.ov-pos {
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.35);
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.ov-pos.ov-next {
  color: #39c5bb;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-align: left;
}

/* 歌名多为日文，优先 JP 字形 */
.ov-name {
  font-size: 15px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: #ffffff;
  font-family: "Yu Gothic UI", "Yu Gothic", "Meiryo UI", "Meiryo", "Microsoft YaHei UI", sans-serif;
}
.ov-song.playing .ov-name { font-size: 16px; }

/* 星级：等宽数字 + 档位色（叠加场景提亮版） */
.ov-stars {
  font-family: "Cascadia Mono", Consolas, monospace;
  font-size: 13px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  color: rgba(255, 255, 255, 0.65);
}
.ov-stars.tier-easy { color: #25b8e6; }
.ov-stars.tier-normal { color: #38d21f; }
.ov-stars.tier-hard { color: #f0b41d; }
.ov-stars.tier-extreme { color: #ff3b57; }
.ov-stars.tier-exextreme { color: #c55aff; }

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
  background: rgba(225, 40, 133, 0.16);
  border: 1px solid rgba(225, 40, 133, 0.4);
  animation: ov-fade-in 200ms ease both;
  overflow: hidden;
}

.ov-failure-requester {
  font-size: 12px;
  font-weight: 600;
  color: #f27eae;
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
