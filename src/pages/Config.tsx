import { useEffect, useMemo, useRef, useState } from 'react';
import { api, emptyConfig } from '../lib/tauri';
import { reportStatus } from '../lib/status';
import type { AppConfig, HotkeyStatus, OBSOverlayStatus, RebuildReport, SongInfo } from '../types';

// 跨页面切换串行保存，重新进入设置时先等待上一页的失焦保存完成。
let configSaveQueue: Promise<boolean> = Promise.resolve(true);

export default function ConfigPage() {
  const [config, setConfig] = useState<AppConfig>(emptyConfig);
  const [savedConfig, setSavedConfig] = useState<AppConfig>(emptyConfig);
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null);
  const [obsStatus, setObsStatus] = useState<OBSOverlayStatus | null>(null);
  const [rebuildReport, setRebuildReport] = useState<RebuildReport | null>(null);
  const [songs, setSongs] = useState<SongInfo[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState('');
  const draftConfig = useRef(emptyConfig);
  const persistedConfig = useRef(emptyConfig);
  const pendingSave = useRef<{ config: AppConfig; promise: Promise<boolean> } | null>(null);

  const hasConfigChanges = useMemo(() => JSON.stringify(config) !== JSON.stringify(savedConfig), [config, savedConfig]);
  const libraryStats = useMemo(() => {
    const translated = songs.filter((song) => song.name_zh).length;
    const modSongs = songs.filter((song) => song.source && song.source !== 'base').length;
    const authors = new Set(songs.flatMap((song) => song.authors.filter(Boolean))).size;
    return { total: songs.length, translated, modSongs, authors };
  }, [songs]);

  useEffect(() => {
    let active = true;
    configSaveQueue.then(() => api.getConfig()).then((loadedConfig) => {
      if (!active) return;
      draftConfig.current = loadedConfig;
      persistedConfig.current = loadedConfig;
      setConfig(loadedConfig);
      setSavedConfig(loadedConfig);
      setLoaded(true);
    }).catch((error) => {
      if (active) reportStatus(`读取配置失败：${String(error)}`);
    });
    api.getHotkeyStatus().then(setHotkeyStatus).catch(() => undefined);
    api.getObsOverlayStatus().then(setObsStatus).catch(() => undefined);
    api.getAllSongs().then(setSongs).catch(() => undefined);
    return () => { active = false; };
  }, []);

  function update<K extends keyof AppConfig>(key: K, value: AppConfig[K], immediate = false) {
    draftConfig.current = { ...draftConfig.current, [key]: value };
    setConfig(draftConfig.current);
    if (immediate) void save();
  }

  function save(): Promise<boolean> {
    if (!loaded) return Promise.resolve(false);
    const snapshot = draftConfig.current;
    const serialized = JSON.stringify(snapshot);
    if (pendingSave.current && JSON.stringify(pendingSave.current.config) === serialized) {
      return pendingSave.current.promise;
    }
    if (!pendingSave.current && JSON.stringify(persistedConfig.current) === serialized) {
      setSaveError('');
      if (saveError) reportStatus('配置与已保存内容一致');
      return Promise.resolve(true);
    }

    setSaving(true);
    setSaveError('');
    const job: { config: AppConfig; promise: Promise<boolean> } = {
      config: snapshot,
      promise: configSaveQueue.then(async () => {
        try {
          if (JSON.stringify(persistedConfig.current) !== serialized) {
            await api.saveConfig(snapshot);
            persistedConfig.current = snapshot;
            setSavedConfig(snapshot);
          }
          if (pendingSave.current === job) {
            setSaveError('');
            reportStatus('配置已自动保存');
          }
          return true;
        } catch (error) {
          if (pendingSave.current === job) {
            const message = `自动保存失败：${String(error)}`;
            setSaveError(message);
            reportStatus(message);
          }
          return false;
        } finally {
          if (pendingSave.current === job) {
            pendingSave.current = null;
            setSaving(false);
          }
        }
      }),
    };
    pendingSave.current = job;
    configSaveQueue = job.promise;
    return job.promise;
  }

  async function rebuildDatabase() {
    try {
      if (!await save()) return;
      const report = await api.rebuildDatabase();
      setRebuildReport(report);
      const updatedSongs = await api.getAllSongs();
      setSongs(updatedSongs);
      reportStatus(`歌曲数据库已重建：${report.total} 首，扫描 MOD ${report.mods_scanned} 个`);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function registerHotkey() {
    try {
      if (!await save()) return;
      const status = await api.registerHotkey();
      setHotkeyStatus(status);
      reportStatus(status.message);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function unregisterHotkey() {
    try {
      const status = await api.unregisterHotkey();
      setHotkeyStatus(status);
      reportStatus(status.message);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function startObs() {
    try {
      if (!await save()) return;
      const status = await api.startObsOverlay();
      setObsStatus(status);
      reportStatus(`${status.message}: ${status.url}`);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  async function stopObs() {
    try {
      const status = await api.stopObsOverlay();
      setObsStatus(status);
      reportStatus(status.message);
    } catch (error) {
      reportStatus(String(error));
    }
  }

  return (
    <section className="page">
      <header className="toolbar">
        <span className="hint">输入框失焦、开关或单选切换后自动保存；连接类设置需重启对应服务生效</span>
        <div className="toolbar-actions" role="status" aria-live="polite">
          {!loaded ? <span className="signal busy"><i />读取配置中</span>
            : saving ? <span className="signal busy"><i />正在保存</span>
            : saveError ? <span className="signal bad" title={saveError}><i />保存失败，请修改后重试</span>
            : hasConfigChanges ? <span className="signal warn"><i />编辑中，失焦后保存</span>
            : <span className="signal ok"><i />已保存</span>}
        </div>
      </header>

      <fieldset className="settings-grid" disabled={!loaded} onBlur={(event) => {
        if (event.target instanceof HTMLInputElement && event.target.type !== 'checkbox') void save();
      }}>
        <ConfigPanel index="01" title="基础设置">
          <Field label="直播间 ID"><input value={config.room_id || ''} inputMode="numeric" pattern="[0-9]*" placeholder="输入 B 站直播间 ID" onChange={(event) => update('room_id', parseRoomId(event.target.value))} /></Field>
          <Field label="切歌快捷键"><input value={config.hotkey} onChange={(event) => update('hotkey', event.target.value)} /></Field>
          <div className="inline-actions">
            <button type="button" className="secondary-button button-sm" onClick={registerHotkey}>注册快捷键</button>
            <button type="button" className="ghost-button button-sm" onClick={unregisterHotkey}>停止快捷键</button>
          </div>
          <p className="hint">{hotkeyStatus?.message ?? '快捷键状态读取中…'}</p>
          <Field label="点歌前缀"><input value={config.song_command_prefix} onChange={(event) => update('song_command_prefix', event.target.value)} /></Field>
          <Toggle label="保存日志到文件" checked={config.log_to_file} onChange={(value) => update('log_to_file', value, true)} />
          <p className="hint">开启后，日志将写入数据目录下的 logs/ 文件夹，按日期分文件保存。</p>
        </ConfigPanel>

        <ConfigPanel index="02" title="队列设置">
          <Field label="最大队列长度"><input value={config.max_queue_size} type="number" min={1} max={200} onChange={(event) => update('max_queue_size', Number(event.target.value))} /></Field>
          <Toggle label="允许重复点歌" checked={config.allow_duplicates} onChange={(value) => update('allow_duplicates', value, true)} />
        </ConfigPanel>

        <ConfigPanel index="03" title="歌曲库">
          <div className="stat-row">
            <div className="stat"><strong>{libraryStats.total}</strong><span>歌曲总数</span></div>
            <div className="stat"><strong>{libraryStats.translated}</strong><span>中文名</span></div>
            <div className="stat"><strong>{libraryStats.modSongs}</strong><span>MOD 歌曲</span></div>
            <div className="stat"><strong>{libraryStats.authors}</strong><span>作者</span></div>
          </div>
          <Field label="数据目录"><input value={config.data_dir} onChange={(event) => update('data_dir', event.target.value)} /></Field>
          <Field label="游戏 MOD 路径"><input value={config.mods_dir} placeholder="例如 D:\SteamLibrary\...\mods" onChange={(event) => update('mods_dir', event.target.value)} /></Field>
          <div className="inline-actions">
            <button type="button" className="secondary-button button-sm" onClick={rebuildDatabase}>重建歌曲库</button>
          </div>
          <p className="hint">路径输入框失焦后自动保存，重建会等待保存成功，再扫描基础曲库与游戏 MOD 目录。</p>
        </ConfigPanel>

        <ConfigPanel index="04" title="难度偏好">
          <Field label="偏好难度">
            <SegmentedControl
              value={config.default_search_difficulty}
              onChange={(v) => update('default_search_difficulty', v, true)}
              options={[
                { value: 'easy', label: '简单' },
                { value: 'normal', label: '普通' },
                { value: 'hard', label: '困难' },
                { value: 'extreme', label: '极限' },
                { value: 'exextreme', label: 'EX极限' },
              ]}
            />
          </Field>
          <Field label="偏好难度不存在时">
            <SegmentedControl
              value={config.difficulty_fallback}
              onChange={(v) => update('difficulty_fallback', v, true)}
              options={[
                { value: 'easier', label: '更简单' },
                { value: 'harder', label: '更难' },
              ]}
            />
          </Field>
          <p className="hint">点歌时优先匹配偏好难度的星级；该难度无数据时按回退方向查找相邻难度。</p>
        </ConfigPanel>

        <ConfigPanel index="05" title="OBS 覆盖层">
          <Toggle label="启用 OBS 覆盖层" checked={config.obs_overlay_enabled} onChange={(value) => update('obs_overlay_enabled', value, true)} />
          <Field label="OBS 地址"><input value={config.obs_overlay_host} onChange={(event) => update('obs_overlay_host', event.target.value)} /></Field>
          <Field label="OBS 端口"><input value={config.obs_overlay_port} type="number" onChange={(event) => update('obs_overlay_port', Number(event.target.value))} /></Field>
          <Field label="OBS 标题"><input value={config.obs_overlay_title} onChange={(event) => update('obs_overlay_title', event.target.value)} /></Field>
          <div className="inline-actions">
            <button type="button" className="secondary-button button-sm" onClick={startObs}>启动 OBS 覆盖层</button>
            <button type="button" className="ghost-button button-sm" onClick={stopObs}>停止</button>
          </div>
          <p className="hint">{obsStatus ? `${obsStatus.message} · ${obsStatus.url}` : 'OBS 覆盖层状态读取中…'}</p>
        </ConfigPanel>

        <ConfigPanel index="06" title="LLM 意图识别">
          <Toggle label="启用 LLM 意图识别" checked={config.llm_enabled} onChange={(value) => update('llm_enabled', value, true)} />
          <p className="hint">启用后，非前缀弹幕会走 OpenAI 兼容接口；本地模型可留空 API Key，云端服务通常需要填写。</p>
          <Field label="API Key（可选）"><input value={config.llm_api_key} type="password" placeholder="本地模型通常可留空" onChange={(event) => update('llm_api_key', event.target.value)} /></Field>
          <Field label="Base URL"><input value={config.llm_base_url} placeholder="https://api.deepseek.com 或 http://127.0.0.1:11434" onChange={(event) => update('llm_base_url', event.target.value)} /></Field>
          <Field label="模型"><input value={config.llm_model} onChange={(event) => update('llm_model', event.target.value)} /></Field>
          <Field label="Max Tokens"><input value={config.llm_max_tokens ?? ''} type="number" min={1} placeholder="留空 = 不限制" onChange={(event) => update('llm_max_tokens', parseMaxTokens(event.target.value))} /></Field>
          <p className="hint">限制 LLM 响应的最大 token 数，留空不限制。点歌意图识别通常 150 足够。</p>
        </ConfigPanel>
      </fieldset>

      {rebuildReport ? (
        <div className="report-panel">
          <h2>数据库重建报告</h2>
          <div className="report-grid">
            <span>歌曲总数</span><strong>{rebuildReport.total}</strong>
            <span>主库导入</span><strong>{rebuildReport.base_imported}</strong>
            <span>MOD 扫描</span><strong>{rebuildReport.mods_scanned}</strong>
            <span>MOD 歌曲</span><strong>{rebuildReport.mods_imported}</strong>
            <span>别名导入</span><strong>{rebuildReport.aliases_imported}</strong>
            <span>中文名合并</span><strong>{rebuildReport.chinese_names_merged}</strong>
            <span>数据目录</span><strong>{rebuildReport.data_dir}</strong>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function ConfigPanel({ index, title, children }: { index: string; title: string; children: React.ReactNode }) {
  return (
    <div className="config-panel">
      <h2><span className="panel-index">{index}</span>{title}</h2>
      {children}
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  // 用 <div> 而非 <label>，避免点击标签把焦点转给第一个控件，
  // 那会破坏 SegmentedControl 的交互。
  return <div className="field"><span>{label}</span>{children}</div>;
}

function parseRoomId(value: string) {
  const digits = value.replace(/\D/g, '');
  return digits ? Number(digits) : 0;
}

function parseMaxTokens(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const num = Number(trimmed);
  return num > 0 ? num : null;
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return (
    <label className="toggle-row">
      <span>{label}</span>
      <input type="checkbox" className="switch" checked={checked} onChange={(event) => onChange(event.target.checked)} />
    </label>
  );
}

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
