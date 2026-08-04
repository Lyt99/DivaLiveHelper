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
  state = { ...state, ...patch };
  emit();
}

/** 从后端拉取最新连接状态并同步到状态栏 */
export async function refreshConnectionState() {
  const [game, danmaku] = await Promise.all([api.getGameConnectionStatus(), api.getDanmakuStatus()]);
  setConnectionState({ gameConnected: game, danmakuConnected: danmaku.connected, roomId: danmaku.room_id });
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
