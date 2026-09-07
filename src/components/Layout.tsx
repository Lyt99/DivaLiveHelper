import { NavLink } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { MouseEvent } from 'react';
import packageJson from '../../package.json';
import ThemeToggle from './ThemeToggle';
import { useShellState } from '../lib/status';

const NoteIcon = (
  <svg width="14" height="14" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M16 3v10" /><path d="M16 3 8 5v10" /><circle cx="14" cy="14" r="2" /><circle cx="6" cy="16" r="2" />
  </svg>
);

const MinIcon = (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" aria-hidden="true">
    <path d="M2.5 6h7" />
  </svg>
);

const MaxIcon = (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.1" aria-hidden="true">
    <rect x="2.5" y="2.5" width="7" height="7" rx="1" />
  </svg>
);

const CloseIcon = (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.1" strokeLinecap="round" aria-hidden="true">
    <path d="M3 3l6 6M9 3l-6 6" />
  </svg>
);

/* 标签导航：mono 序号是控制台的设计语言 */
const navItems: { to: string; label: string; index: string }[] = [
  { to: '/', label: '点歌台', index: '01' },
  { to: '/library', label: '歌曲库', index: '02' },
  { to: '/config', label: '设置', index: '03' },
  { to: '/logs', label: '日志', index: '04' },
  { to: '/about', label: '关于', index: '05' },
];

interface LayoutProps {
  children: React.ReactNode;
}

export default function Layout({ children }: LayoutProps) {
  const appWindow = getCurrentWindow();
  const shell = useShellState();

  const startWindowDrag = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    void appWindow.startDragging();
  };

  return (
    <div className="app-frame">
      <header className="topbar" onMouseDown={startWindowDrag}>
        <div className="topbar-brand">
          {NoteIcon}
          <span>DIVA 直播助手</span>
        </div>
        <nav className="topbar-nav" onMouseDown={(event) => event.stopPropagation()}>
          {navItems.map((item) => (
            <NavLink key={item.to} to={item.to} end={item.to === '/'} className={({ isActive }) => `topbar-tab ${isActive ? 'active' : ''}`}>
              <span className="tab-index">{item.index}</span>
              <span>{item.label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="window-controls" onMouseDown={(event) => event.stopPropagation()}>
          <button type="button" aria-label="最小化" onClick={() => appWindow.minimize()}>{MinIcon}</button>
          <button type="button" aria-label="最大化" onClick={() => appWindow.toggleMaximize()}>{MaxIcon}</button>
          <button type="button" aria-label="关闭" className="close" onClick={() => appWindow.close()}>{CloseIcon}</button>
        </div>
      </header>

      <main className="content">{children}</main>

      <footer className="statusbar">
        <span className={`signal ${shell.danmakuConnected ? 'ok' : 'bad'}`}>
          <i />
          直播间
          {shell.danmakuConnected && shell.roomId ? <span className="mono">#{shell.roomId}</span> : '未连接'}
        </span>
        <span className={`signal ${shell.gameConnected ? 'ok' : 'bad'}`}>
          <i />
          游戏{shell.gameConnected ? '运行中' : '未启动'}
        </span>
        <div className="statusbar-right">
          <span className="statusbar-echo" aria-live="polite">{shell.message}</span>
          <span className="statusbar-version">v{packageJson.version}</span>
          <ThemeToggle />
        </div>
      </footer>
    </div>
  );
}
