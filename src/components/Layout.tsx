import { NavLink } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { MouseEvent, ReactNode } from 'react';
import ThemeToggle from './ThemeToggle';

const SongIcon = (
  <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M16 3v10" /><path d="M16 3 8 5v10" /><circle cx="14" cy="14" r="2" /><circle cx="6" cy="16" r="2" />
  </svg>
);

const LibraryIcon = (
  <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <rect x="3" y="3" width="6" height="6" rx="0.5" /><rect x="11" y="3" width="6" height="6" rx="0.5" /><rect x="3" y="11" width="6" height="6" rx="0.5" /><rect x="11" y="11" width="6" height="6" rx="0.5" />
  </svg>
);

const ConfigIcon = (
  <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <line x1="3" y1="5" x2="17" y2="5" /><line x1="3" y1="10" x2="17" y2="10" /><line x1="3" y1="15" x2="17" y2="15" /><circle cx="7" cy="5" r="1.5" /><circle cx="13" cy="10" r="1.5" /><circle cx="9" cy="15" r="1.5" />
  </svg>
);

const LogsIcon = (
  <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <rect x="3" y="4" width="14" height="12" rx="1.5" /><path d="M6 9l2 2-2 2" /><line x1="10" y1="13" x2="14" y2="13" />
  </svg>
);

const navItems: { to: string; label: string; icon: ReactNode }[] = [
  { to: '/', label: '点歌', icon: SongIcon },
  { to: '/library', label: '歌曲库', icon: LibraryIcon },
  { to: '/config', label: '设置', icon: ConfigIcon },
  { to: '/logs', label: '日志', icon: LogsIcon },
];

interface LayoutProps {
  children: React.ReactNode;
}

export default function Layout({ children }: LayoutProps) {
  const appWindow = getCurrentWindow();
  const startWindowDrag = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    void appWindow.startDragging();
  };

  return (
    <div className="app-frame">
      <header className="titlebar" onMouseDown={startWindowDrag}>
        <div className="titlebar-left">
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M16 3v10" /><path d="M16 3 8 5v10" /><circle cx="14" cy="14" r="2" /><circle cx="6" cy="16" r="2" />
          </svg>
          <span>DIVA 直播助手</span>
        </div>
        <div className="window-controls" onMouseDown={(event) => event.stopPropagation()}>
          <button type="button" aria-label="最小化" onClick={() => appWindow.minimize()}>—</button>
          <button type="button" aria-label="最大化" onClick={() => appWindow.toggleMaximize()}>□</button>
          <button type="button" aria-label="关闭" className="close" onClick={() => appWindow.close()}>×</button>
        </div>
      </header>
      <div className="shell">
        <aside className="sidebar">
          <div className="brand">
            <div className="brand-mark">DL</div>
            <div>
              <div className="brand-title">DIVA Live</div>
              <div className="brand-subtitle">点歌控制台</div>
            </div>
          </div>
          <nav className="nav-list">
            {navItems.map((item) => (
              <NavLink key={item.to} to={item.to} end={item.to === '/'} className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}>
                <span className="nav-icon">{item.icon}</span>
                <span>{item.label}</span>
              </NavLink>
            ))}
          </nav>
          <div className="sidebar-footer">
            <ThemeToggle />
            <div className="mini-note">Powered by Milkchan</div>
          </div>
        </aside>
        <main className="content">{children}</main>
      </div>
    </div>
  );
}
