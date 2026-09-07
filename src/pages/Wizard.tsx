import { useEffect, useRef, useState, type Dispatch, type MouseEvent, type RefObject, type SetStateAction } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { open } from '@tauri-apps/plugin-dialog';
import { api, emptyConfig } from '../lib/tauri';
import type { AppConfig, GameInstallation, RebuildReport } from '../types';

const TOTAL_STEPS = 7;

const DIFFICULTY_OPTIONS = [
  { value: 'easy', label: '简单' },
  { value: 'normal', label: '普通' },
  { value: 'hard', label: '困难' },
  { value: 'extreme', label: '极限' },
  { value: 'exextreme', label: 'EX' },
];

const FALLBACK_OPTIONS = [
  { value: 'easier', label: '更简单' },
  { value: 'harder', label: '更难' },
];

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

interface StepProps {
  config: AppConfig;
  setConfig: Dispatch<SetStateAction<AppConfig>>;
  next: () => void;
  back: () => void;
}

export default function Wizard() {
  const [step, setStep] = useState(0);
  const [config, setConfig] = useState<AppConfig>({ ...emptyConfig });
  const [configLoading, setConfigLoading] = useState(true);
  const [configError, setConfigError] = useState('');
  const modsDirEdited = useRef(false);
  const [rebuildReport, setRebuildReport] = useState<RebuildReport | null>(null);
  const appWindow = getCurrentWindow();

  const startWindowDrag = (event: MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    void appWindow.startDragging();
  };

  useEffect(() => {
    let cancelled = false;
    api.getConfig().then(
      (savedConfig) => {
        if (cancelled) return;
        setConfig(savedConfig);
        setConfigLoading(false);
      },
      (error) => {
        if (cancelled) return;
        setConfigError(String(error));
        setConfigLoading(false);
      },
    );
    return () => { cancelled = true; };
  }, []);

  const go = (n: number) => setStep(n);
  const next = () => go(step + 1);
  const back = () => go(Math.max(0, step - 1));

  const finish = async () => {
    try {
      // 跳过校验 —— 房间号允许为 0（用户可能跳过了该步骤）
      await api.saveConfig({ ...config }, true);
      window.location.reload();
    } catch (e) {
      alert(`保存配置失败: ${e}`);
    }
  };

  return (
    <div className="wizard">
      <header className="topbar wizard-titlebar" onMouseDown={startWindowDrag}>
        <div className="topbar-brand">
          <span>DIVA 直播助手 · 初始配置</span>
        </div>
        <div className="window-controls" onMouseDown={(event) => event.stopPropagation()}>
          <button type="button" aria-label="最小化" onClick={() => appWindow.minimize()}>{MinIcon}</button>
          <button type="button" aria-label="最大化" onClick={() => appWindow.toggleMaximize()}>{MaxIcon}</button>
          <button type="button" aria-label="关闭" className="close" onClick={() => appWindow.close()}>{CloseIcon}</button>
        </div>
      </header>

      <div className="wizard-progress">
        <span className="wizard-progress-label">{String(step + 1).padStart(2, '0')} / {String(TOTAL_STEPS).padStart(2, '0')}</span>
        <div className="wizard-progress-track">
          <div className="wizard-progress-fill" style={{ width: `${((step + 1) / TOTAL_STEPS) * 100}%` }} />
        </div>
      </div>

      <div className="wizard-body">
        {step === 0 && <WelcomeStep next={next} loading={configLoading} error={configError} />}
        {step === 1 && <ModsDirStep config={config} setConfig={setConfig} edited={modsDirEdited} next={next} back={back} />}
        {step === 2 && <RebuildStep config={config} setConfig={setConfig} report={rebuildReport} setReport={setRebuildReport} next={next} back={back} />}
        {step === 3 && <RoomIdStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 4 && <PrefixStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 5 && <DifficultyStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 6 && <HotkeyStep config={config} setConfig={setConfig} next={finish} back={back} />}
      </div>
    </div>
  );
}

/* ── 第 1 步：欢迎 ────────────────────────────── */

function WelcomeStep({ next, loading, error }: { next: () => void; loading: boolean; error: string }) {
  return (
    <div className="wizard-step wizard-welcome">
      <p className="wizard-kicker">首次运行 · 快速配置</p>
      <h1>DIVA 直播助手</h1>
      <div className="wizard-rule" />
      <p className="muted">
        监听 B 站直播间弹幕，自动识别点歌请求，
        管理歌曲队列，一键切换游戏内曲目。
      </p>
      <p className="hint">接下来几步帮你完成基础配置，大约 1 分钟。</p>
      {error && <p className="error-text" role="alert">读取配置失败：{error}。可以使用默认值继续配置。</p>}
      <div className="wizard-actions">
        <button className="primary-button" onClick={next} disabled={loading}>
          {loading ? '正在读取配置…' : error ? '使用默认值开始配置' : '开始配置'}
        </button>
      </div>
    </div>
  );
}

/* ── 第 2 步：游戏 MOD 目录 ───────────────────── */

type GameDiscovery =
  | { status: 'searching' | 'existing' | 'manual' | 'missing' }
  | { status: 'found'; installation: GameInstallation }
  | { status: 'error'; error: string };

function ModsDirStep({ config, setConfig, edited, next, back }: StepProps & { edited: RefObject<boolean> }) {
  const request = useRef(0);
  const [discovery, setDiscovery] = useState<GameDiscovery>(() => ({
    status: config.mods_dir.trim() ? 'existing' : edited.current ? 'manual' : 'searching',
  }));

  useEffect(() => {
    const currentRequest = ++request.current;
    if (!config.mods_dir.trim() && !edited.current) {
      api.detectGameInstallation().then(
        (installation) => {
          if (request.current !== currentRequest) return;
          setDiscovery(installation ? { status: 'found', installation } : { status: 'missing' });
          if (installation?.mods_dir) {
            const modsDir = installation.mods_dir;
            setConfig((current) => current.mods_dir.trim() ? current : { ...current, mods_dir: modsDir });
          }
        },
        (error) => {
          if (request.current !== currentRequest) return;
          setDiscovery({ status: 'error', error: String(error) });
        },
      );
    }
    return () => { request.current++; };
  }, [config.mods_dir, edited, setConfig]);

  const beginManualEdit = () => {
    edited.current = true;
    setDiscovery({ status: 'manual' });
    return ++request.current;
  };

  const pick = async () => {
    const currentRequest = beginManualEdit();
    try {
      const path = await open({ directory: true, multiple: false });
      if (path && request.current === currentRequest) {
        setConfig((current) => ({ ...current, mods_dir: path }));
      }
    } catch { /* 用户取消 */ }
  };

  const leave = (navigate: () => void) => {
    request.current++;
    navigate();
  };

  return (
    <div className="wizard-step">
      <h2>自动发现游戏与 MOD 目录</h2>
      <p className="muted">
        自动查找 Steam 库中已安装的 Project DIVA Mega Mix Plus。
        MOD 歌曲信息会从 <code>mods/</code> 下各模组的 <code>rom/mod_pv_db.txt</code> 中读取。
      </p>
      <div aria-live="polite" aria-busy={discovery.status === 'searching'}>
        <p className={discovery.status === 'error' ? 'error-text' : 'hint'}>
          {discovery.status === 'searching' && '正在查找 Steam 游戏安装目录…你也可以手动填写或留空继续。'}
          {discovery.status === 'existing' && '已保留现有 MOD 目录，不会自动覆盖。'}
          {discovery.status === 'manual' && '已切换为手动设置，不会自动覆盖你的修改。'}
          {discovery.status === 'missing' && '未找到 Steam 或已安装的游戏。可以手动选择 MOD 目录，或留空使用官方曲库。'}
          {discovery.status === 'found' && (discovery.installation.mods_dir
            ? '已找到游戏并自动填入 MOD 目录。'
            : '已找到游戏，但尚无 mods 文件夹。不会自动创建；可以手动选择其他 MOD 目录，或留空使用官方曲库。')}
          {discovery.status === 'error' && `自动发现失败：${discovery.error}。可以手动选择 MOD 目录，或留空使用官方曲库。`}
        </p>
        {discovery.status === 'found' && (
          <p className="hint wizard-game-path">游戏目录：<code>{discovery.installation.game_dir}</code></p>
        )}
      </div>
      <div className="wizard-field-row">
        <input
          type="text"
          aria-label="游戏 MOD 目录"
          value={config.mods_dir}
          onChange={(e) => {
            const modsDir = e.target.value;
            beginManualEdit();
            setConfig((current) => ({ ...current, mods_dir: modsDir }));
          }}
          placeholder="例如 D:\SteamLibrary\...\mods"
        />
        <button className="secondary-button" onClick={pick}>浏览</button>
      </div>
      <p className="hint">不使用 MOD 歌曲时，可以留空进入下一步，继续导入官方曲库。</p>
      <div className="wizard-actions">
        <button className="ghost-button" onClick={() => leave(back)}>上一步</button>
        <button className="primary-button" onClick={() => leave(next)}>下一步</button>
      </div>
    </div>
  );
}

/* ── 第 3 步：重建歌曲库 ──────────────────────── */

function RebuildStep({ config, report, setReport, next, back }: StepProps & { report: RebuildReport | null; setReport: (r: RebuildReport | null) => void }) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const rebuild = async () => {
    setLoading(true);
    setError('');
    setReport(null);
    try {
      // 先保存向导配置，让后端拿到 mods_dir（跳过校验 —— 此时房间号可能还是 0）
      await api.saveConfig({ ...config }, true);
      const r = await api.rebuildDatabase();
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="wizard-step">
      <h2>刷新歌曲库</h2>
      <p className="muted">扫描基础曲库与 MOD 目录，构建搜索索引。</p>
      {report ? (
        <div className="wizard-report">
          <div className="wizard-report-row"><span>基础歌曲</span><span>{report.base_imported} 首</span></div>
          <div className="wizard-report-row"><span>MOD 歌曲</span><span>{report.mods_imported} 首</span></div>
          <div className="wizard-report-row"><span>别名导入</span><span>{report.aliases_imported} 条</span></div>
          <div className="wizard-report-row"><span>中文名库</span><span>{report.chinese_names_total} 条（新增 {report.chinese_names_merged}）</span></div>
          <div className="wizard-report-total">共 {report.total} 首</div>
        </div>
      ) : error ? (
        <p className="error-text">{error}</p>
      ) : null}
      <div className="wizard-actions">
        <button className="ghost-button" onClick={back}>上一步</button>
        <button className="secondary-button" onClick={rebuild} disabled={loading}>
          {loading ? '扫描中…' : report ? '重新扫描' : '开始扫描'}
        </button>
        <button className="primary-button" onClick={next} disabled={!report && !error}>
          下一步
        </button>
      </div>
    </div>
  );
}

/* ── 第 4 步：直播间房间号（可跳过） ───────────── */

function RoomIdStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>直播间房间号</h2>
      <p className="muted">
        填入你的 B 站直播间 ID，启动后可以自动接收弹幕点歌。
        也可以稍后在设置中填写。
      </p>
      <input
        type="number"
        value={config.room_id || ''}
        onChange={(e) => setConfig({ ...config, room_id: Number(e.target.value) || 0 })}
        placeholder="例如 21452505"
        min={0}
      />
      <div className="wizard-actions">
        <button className="ghost-button" onClick={back}>上一步</button>
        <button className="ghost-button" onClick={next}>跳过</button>
        <button className="primary-button" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── 第 5 步：点歌指令前缀 ────────────────────── */

function PrefixStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>点歌指令前缀</h2>
      <p className="muted">
        弹幕以此前缀开头会被识别为点歌指令。
        例如前缀为「点歌」时，弹幕「点歌 告白」会触发搜索。
      </p>
      <input
        type="text"
        value={config.song_command_prefix}
        onChange={(e) => setConfig({ ...config, song_command_prefix: e.target.value })}
        placeholder="点歌"
      />
      <div className="wizard-actions">
        <button className="ghost-button" onClick={back}>上一步</button>
        <button className="primary-button" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── 第 6 步：难度偏好 ────────────────────────── */

function DifficultyStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>难度偏好</h2>
      <p className="muted">
        点歌时优先匹配该难度下的星级。若歌曲在该难度下无数据，则按回退方向查找。
      </p>
      <div className="wizard-field">
        <label>偏好难度</label>
        <SegmentedControl
          value={config.default_search_difficulty}
          onChange={(v) => setConfig({ ...config, default_search_difficulty: v })}
          options={DIFFICULTY_OPTIONS}
        />
      </div>
      <div className="wizard-field">
        <label>偏好难度不存在时</label>
        <SegmentedControl
          value={config.difficulty_fallback}
          onChange={(v) => setConfig({ ...config, difficulty_fallback: v })}
          options={FALLBACK_OPTIONS}
        />
      </div>
      <div className="wizard-actions">
        <button className="ghost-button" onClick={back}>上一步</button>
        <button className="primary-button" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── 第 7 步：全局快捷键 ──────────────────────── */

function HotkeyStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>全局快捷键</h2>
      <p className="muted">
        按下快捷键自动切到队列下一首歌。
        格式：修饰键+按键，如 <code>ctrl+shift+n</code>。
      </p>
      <input
        type="text"
        value={config.hotkey}
        onChange={(e) => setConfig({ ...config, hotkey: e.target.value })}
        placeholder="ctrl+shift+n"
      />
      <p className="hint">注册全局快捷键可能需要管理员权限。</p>
      <div className="wizard-actions">
        <button className="ghost-button" onClick={back}>上一步</button>
        <button className="primary-button" onClick={next}>完成配置</button>
      </div>
    </div>
  );
}

/* ── 共用：分段控件 ── */

function SegmentedControl({ value, onChange, options }: { value: string; onChange: (value: string) => void; options: { value: string; label: string }[] }) {
  return (
    <div className="segmented-control">
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          className={`segmented-btn${opt.value === value ? ' active' : ''}`}
          onClick={() => onChange(opt.value)}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}
