import { open } from '@tauri-apps/plugin-shell';
import packageJson from '../../package.json';
import { reportStatus } from '../lib/status';

const REPO_URL = 'https://github.com/Lyt99/DivaLiveHelper';
const PREDECESSOR_URL = 'https://github.com/hiki8man/DivaDanmuSelecter';

const ArrowIcon = (
  <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M3 9 9 3M4.5 3H9v4.5" />
  </svg>
);

export default function AboutPage() {
  const openLink = (url: string) => {
    open(url).catch(() => reportStatus('无法打开链接'));
  };

  return (
    <section className="page about-page">
      <div className="about-identity">
        <div className="about-name">DIVA 直播助手</div>
        <p className="about-tagline">B 站直播点歌控制台 · 为 Project DIVA Mega Mix Plus 而设计</p>
      </div>

      <div className="about-section">
        <h2><span className="panel-index">01</span>应用信息</h2>
        <div className="about-rows">
          <div className="about-row"><span>版本</span><strong className="mono">v{packageJson.version}</strong></div>
          <div className="about-row"><span>作者</span><strong>Milkchan</strong></div>
          <div className="about-row"><span>技术栈</span><strong className="mono">Rust + Tauri 2 + React</strong></div>
          <div className="about-row"><span>平台</span><strong>Windows</strong></div>
        </div>
      </div>

      <div className="about-section">
        <h2><span className="panel-index">02</span>链接</h2>
        <div className="about-links">
          <button type="button" className="about-link" onClick={() => openLink(REPO_URL)} title="在浏览器中打开项目仓库">
            <span>项目仓库</span>
            <span className="mono">github.com/Lyt99/DivaLiveHelper</span>
            {ArrowIcon}
          </button>
          <button type="button" className="about-link" onClick={() => openLink(PREDECESSOR_URL)} title="在浏览器中打开前身项目">
            <span>灵感来源</span>
            <span className="mono">DivaDanmuSelecter · hiki8man</span>
            {ArrowIcon}
          </button>
        </div>
      </div>

      <div className="about-section">
        <h2><span className="panel-index">03</span>免责声明</h2>
        <p className="about-disclaimer">
          本项目是非官方粉丝自制工具，与 SEGA、Crypton Future Media、bilibili 无任何关联。
          本工具通过写入游戏进程内存来切换歌曲，属于对游戏的修改行为，使用风险由使用者自行承担。
          Hatsune Miku Project DIVA 是 SEGA 的商标，初音未来是 Crypton Future Media 的商标。
          本工具仅供个人学习和娱乐使用。
        </p>
      </div>
    </section>
  );
}
