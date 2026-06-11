import { NavLink } from 'react-router-dom';
import ThemeToggle from './ThemeToggle';

const navItems = [
  { to: '/', label: '点歌队列', icon: '♪' },
  { to: '/library', label: '歌曲库', icon: 'DB' },
  { to: '/config', label: '设置', icon: 'CFG' },
  { to: '/logs', label: '日志', icon: 'LOG' },
];

interface LayoutProps {
  children: React.ReactNode;
}

export default function Layout({ children }: LayoutProps) {
  return (
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
          <div className="mini-note">Rust + Tauri 重写预览版</div>
        </div>
      </aside>
      <main className="content">{children}</main>
    </div>
  );
}
