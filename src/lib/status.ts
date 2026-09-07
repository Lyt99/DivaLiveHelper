import { useSyncExternalStore } from 'react';
import { api } from './tauri';

/**
 * 全局外壳状态：底部状态栏显示的连接状态与最近一条事件消息。
 * 任何页面都可以通过 reportStatus / setConnectionState 更新。
 */

export interface ShellState {
  message: string;
  danmakuConnected: boolean;
  roomId: number;
  gameConnected: boolean;
}

let state: ShellState = {
  message: '准备就绪',
  danmakuConnected: false,
  roomId: 0,
  gameConnected: false,
};

const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

/** 更新状态栏右侧的回显消息 */
export function reportStatus(message: string) {
  state = { ...state, message };
  emit();
}

/** 页面拿到最新连接状态后直接推送，避免重复请求后端 */
export function setConnectionState(patch: Partial<Omit<ShellState, 'message'>>) {
  // 定时检测结果未变化时，不重复通知整个界面。
  if (
    (patch.gameConnected ?? state.gameConnected) === state.gameConnected &&
    (patch.danmakuConnected ?? state.danmakuConnected) === state.danmakuConnected &&
    (patch.roomId ?? state.roomId) === state.roomId
  ) return;
  state = { ...state, ...patch };
  emit();
}

/** 弹幕动作后更新状态；游戏进程由主窗口统一定时检测。 */
export async function refreshDanmakuState() {
  const danmaku = await api.getDanmakuStatus();
  setConnectionState({ danmakuConnected: danmaku.connected, roomId: danmaku.room_id });
}

export function useShellState(): ShellState {
  return useSyncExternalStore(
    (callback) => {
      listeners.add(callback);
      return () => {
        listeners.delete(callback);
      };
    },
    () => state,
  );
}
