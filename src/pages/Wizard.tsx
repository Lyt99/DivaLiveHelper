import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { api, emptyConfig } from '../lib/tauri';
import type { AppConfig, RebuildReport } from '../types';

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

interface StepProps {
  config: AppConfig;
  setConfig: (c: AppConfig) => void;
  next: () => void;
  back: () => void;
}

export default function Wizard() {
  const [step, setStep] = useState(0);
  const [config, setConfig] = useState<AppConfig>({ ...emptyConfig });
  const [rebuildReport, setRebuildReport] = useState<RebuildReport | null>(null);

  useEffect(() => {
    api.getConfig().then(setConfig);
  }, []);

  const go = (n: number) => setStep(n);
  const next = () => go(step + 1);
  const back = () => go(Math.max(0, step - 1));

  const finish = async () => {
    try {
      await api.saveConfig({ ...config });
      window.location.reload();
    } catch (e) {
      alert(`保存配置失败: ${e}`);
    }
  };

  return (
    <div className="wizard">
      <div className="wizard-progress">
        {Array.from({ length: 7 }, (_, i) => (
          <div
            key={i}
            className={`wizard-dot ${i <= step ? 'done' : ''} ${i === step ? 'active' : ''}`}
          />
        ))}
      </div>

      <div className="wizard-body">
        {step === 0 && <WelcomeStep next={next} />}
        {step === 1 && <ModsDirStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 2 && <RebuildStep report={rebuildReport} setReport={setRebuildReport} next={next} back={back} />}
        {step === 3 && <RoomIdStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 4 && <PrefixStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 5 && <DifficultyStep config={config} setConfig={setConfig} next={next} back={back} />}
        {step === 6 && <HotkeyStep config={config} setConfig={setConfig} next={finish} back={back} />}
      </div>
    </div>
  );
}

/* ── Step 1: Welcome ────────────────────────────── */

function WelcomeStep({ next }: { next: () => void }) {
  return (
    <div className="wizard-step wizard-welcome">
      <div className="wizard-icon">DL</div>
      <h1>DIVA 直播助手</h1>
      <p className="muted">
        监听 B 站直播间弹幕，自动识别点歌请求，<br />
        管理歌曲队列，一键切换游戏内曲目。
      </p>
      <p className="muted">接下来几步帮你完成基础配置，大约 1 分钟。</p>
      <button className="btn-primary" onClick={next}>开始配置</button>
    </div>
  );
}

/* ── Step 2: Game MOD directory ──────────────────── */

function ModsDirStep({ config, setConfig, next, back }: StepProps) {
  const pick = async () => {
    try {
      const path = await open({ directory: true, multiple: false });
      if (path) setConfig({ ...config, mods_dir: path });
    } catch { /* cancelled */ }
  };

  return (
    <div className="wizard-step">
      <h2>游戏 MOD 目录</h2>
      <p className="muted">
        选择 Project DIVA Mega Mix Plus 的 <code>mods/</code> 文件夹路径。<br />
        MOD 歌曲信息会从该目录下各模组的 <code>rom/mod_pv_db.txt</code> 中读取。
      </p>
      <div className="wizard-field-row">
        <input
          type="text"
          value={config.mods_dir}
          onChange={(e) => setConfig({ ...config, mods_dir: e.target.value })}
          placeholder="例如 D:\SteamLibrary\...\mods"
        />
        <button className="btn-secondary" onClick={pick}>浏览</button>
      </div>
      <p className="hint">如果不使用 MOD 歌曲，可以留空跳过。</p>
      <div className="wizard-actions">
        <button className="btn-ghost" onClick={back}>上一步</button>
        <button className="btn-primary" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── Step 3: Rebuild song database ──────────────── */

function RebuildStep({ report, setReport, next, back }: Omit<StepProps, 'config' | 'setConfig'> & { report: RebuildReport | null; setReport: (r: RebuildReport | null) => void }) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const rebuild = async () => {
    setLoading(true);
    setError('');
    setReport(null);
    try {
      // Save config first so the backend picks up mods_dir (skip validation — room_id may be 0 during wizard)
      await api.saveConfig({ ...(await api.getConfig()) }, true);
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
          <div className="wizard-report-row"><span>中文名合并</span><span>{report.chinese_names_merged} 条</span></div>
          <div className="wizard-report-total">共 {report.total} 首</div>
        </div>
      ) : error ? (
        <p className="error-text">{error}</p>
      ) : null}
      <div className="wizard-actions">
        <button className="btn-ghost" onClick={back}>上一步</button>
        <button className="btn-secondary" onClick={rebuild} disabled={loading}>
          {loading ? '扫描中…' : report ? '重新扫描' : '开始扫描'}
        </button>
        <button className="btn-primary" onClick={next} disabled={!report && !error}>
          下一步
        </button>
      </div>
    </div>
  );
}

/* ── Step 4: Room ID (optional / skippable) ─────── */

function RoomIdStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>直播间房间号</h2>
      <p className="muted">
        填入你的 B 站直播间 ID，启动后可以自动接收弹幕点歌。<br />
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
        <button className="btn-ghost" onClick={back}>上一步</button>
        <button className="btn-ghost" onClick={next}>跳过</button>
        <button className="btn-primary" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── Step 5: Song command prefix ────────────────── */

function PrefixStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>点歌指令前缀</h2>
      <p className="muted">
        弹幕以此前缀开头会被识别为点歌指令。<br />
        例如前缀为「点歌」时，弹幕「点歌 告白」会触发搜索。
      </p>
      <input
        type="text"
        value={config.song_command_prefix}
        onChange={(e) => setConfig({ ...config, song_command_prefix: e.target.value })}
        placeholder="点歌"
      />
      <div className="wizard-actions">
        <button className="btn-ghost" onClick={back}>上一步</button>
        <button className="btn-primary" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── Step 6: Difficulty preference ───────────────── */

function DifficultyStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>难度偏好</h2>
      <p className="muted">
        点歌时优先显示该难度下的星级。若歌曲在该难度下无数据，则按回退策略查找。
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
        <button className="btn-ghost" onClick={back}>上一步</button>
        <button className="btn-primary" onClick={next}>下一步</button>
      </div>
    </div>
  );
}

/* ── Step 7: Hotkey ─────────────────────────────── */

function HotkeyStep({ config, setConfig, next, back }: StepProps) {
  return (
    <div className="wizard-step">
      <h2>全局快捷键</h2>
      <p className="muted">
        按下快捷键自动切到队列下一首歌。<br />
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
        <button className="btn-ghost" onClick={back}>上一步</button>
        <button className="btn-primary" onClick={next}>完成配置</button>
      </div>
    </div>
  );
}

/* ── Shared: SegmentedControl ── */

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
