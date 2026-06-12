import { useEffect, useMemo, useState } from 'react';
import { api, emptyConfig } from '../lib/tauri';
import type { AppConfig, HotkeyStatus, OBSOverlayStatus, RebuildReport, SongInfo } from '../types';

export default function ConfigPage() {
  const [config, setConfig] = useState<AppConfig>(emptyConfig);
  const [savedConfig, setSavedConfig] = useState<AppConfig>(emptyConfig);
  const [hotkeyStatus, setHotkeyStatus] = useState<HotkeyStatus | null>(null);
  const [obsStatus, setObsStatus] = useState<OBSOverlayStatus | null>(null);
  const [rebuildReport, setRebuildReport] = useState<RebuildReport | null>(null);
  const [songs, setSongs] = useState<SongInfo[]>([]);
  const [message, setMessage] = useState('');
  const [toast, setToast] = useState('');

  const hasConfigChanges = useMemo(() => JSON.stringify(config) !== JSON.stringify(savedConfig), [config, savedConfig]);
  const libraryStats = useMemo(() => {
    const translated = songs.filter((song) => song.name_zh).length;
    const modSongs = songs.filter((song) => song.source && song.source !== 'base').length;
    const authors = new Set(songs.flatMap((song) => song.authors.filter(Boolean))).size;
    return { total: songs.length, translated, modSongs, authors };
  }, [songs]);

  useEffect(() => {
    api.getConfig().then((loaded) => {
      setConfig(loaded);
      setSavedConfig(loaded);
    }).catch((error) => setMessage(String(error)));
    api.getHotkeyStatus().then(setHotkeyStatus).catch(() => undefined);
    api.getObsOverlayStatus().then(setObsStatus).catch(() => undefined);
    api.getAllSongs().then(setSongs).catch(() => undefined);
  }, []);

  useEffect(() => {
    if (!toast) {
      return;
    }
    const timer = window.setTimeout(() => setToast(''), 2200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  function update<K extends keyof AppConfig>(key: K, value: AppConfig[K]) {
    setConfig((current) => ({ ...current, [key]: value }));
  }

  async function save() {
    try {
      await api.saveConfig(config);
      const saved = await api.getConfig();
      setConfig(saved);
      setSavedConfig(saved);
      setMessage('配置已保存');
      setToast('保存成功');
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function rebuildDatabase() {
    try {
      const report = await api.rebuildDatabase();
      setRebuildReport(report);
      const updatedSongs = await api.getAllSongs();
      setSongs(updatedSongs);
      setMessage(`歌曲数据库已重建：${report.total} 首，扫描 MOD ${report.mods_scanned} 个`);
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function registerHotkey() {
    try {
      await api.saveConfig(config);
      const status = await api.registerHotkey();
      setHotkeyStatus(status);
      setMessage(status.message);
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function unregisterHotkey() {
    try {
      const status = await api.unregisterHotkey();
      setHotkeyStatus(status);
      setMessage(status.message);
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function startObs() {
    try {
      await api.saveConfig(config);
      const status = await api.startObsOverlay();
      setObsStatus(status);
      setMessage(`${status.message}: ${status.url}`);
    } catch (error) {
      setMessage(String(error));
    }
  }

  async function stopObs() {
    try {
      const status = await api.stopObsOverlay();
      setObsStatus(status);
      setMessage(status.message);
    } catch (error) {
      setMessage(String(error));
    }
  }

  return (
    <section className="page">
      <header className="page-header">
        <div className="page-header-main">
          <p className="eyebrow">Settings</p>
          <h1>可视化配置</h1>
          <p className="muted">替代手动编辑 config.json，保存后部分连接类设置需要重新启动对应服务。</p>
        </div>
        <div className="header-actions">
          <button type="button" className="primary-button" onClick={save} disabled={!hasConfigChanges}>保存设置</button>
          <span className="muted">{message}</span>
        </div>
      </header>

      <div className="settings-grid">
        <ConfigPanel title="基础设置">
          <Field label="直播间 ID"><input value={config.room_id || ''} inputMode="numeric" pattern="[0-9]*" placeholder="输入 B 站直播间 ID" onChange={(event) => update('room_id', parseRoomId(event.target.value))} /></Field>
          <Field label="切歌快捷键"><input value={config.hotkey} onChange={(event) => update('hotkey', event.target.value)} /></Field>
          <div className="inline-actions">
            <button type="button" className="secondary-button" onClick={registerHotkey}>注册快捷键</button>
            <button type="button" className="ghost-button" onClick={unregisterHotkey}>停止快捷键</button>
          </div>
          <p className="notice-text">{hotkeyStatus?.message ?? '快捷键状态读取中…'}</p>
          <Field label="点歌前缀"><input value={config.song_command_prefix} onChange={(event) => update('song_command_prefix', event.target.value)} /></Field>
        </ConfigPanel>

        <ConfigPanel title="队列设置">
          <Field label="最大队列长度"><input value={config.max_queue_size} type="number" min={1} max={200} onChange={(event) => update('max_queue_size', Number(event.target.value))} /></Field>
          <Toggle label="允许重复点歌" checked={config.allow_duplicates} onChange={(value) => update('allow_duplicates', value)} />
          <Toggle label="自动播放下一首" checked={config.auto_play_next} onChange={(value) => update('auto_play_next', value)} />
          <Field label="自动播放间隔（秒）"><input value={config.auto_play_interval} type="number" onChange={(event) => update('auto_play_interval', Number(event.target.value))} /></Field>
        </ConfigPanel>

        <ConfigPanel title="歌曲库">
          <div className="library-stats">
            <div><strong>{libraryStats.total}</strong><span>歌曲总数</span></div>
            <div><strong>{libraryStats.translated}</strong><span>中文名</span></div>
            <div><strong>{libraryStats.modSongs}</strong><span>MOD 歌曲</span></div>
            <div><strong>{libraryStats.authors}</strong><span>作者</span></div>
          </div>
          <Field label="数据目录"><input value={config.data_dir} onChange={(event) => update('data_dir', event.target.value)} /></Field>
          <Field label="游戏 MOD 路径"><input value={config.mods_dir} placeholder="例如 D:\\SteamLibrary\\...\\mods" onChange={(event) => update('mods_dir', event.target.value)} /></Field>
          <div className="inline-actions">
            <button type="button" className="secondary-button" onClick={rebuildDatabase}>重建歌曲库</button>
          </div>
          <p className="notice-text">修改路径后先保存设置，再重建歌曲库。重建会扫描基础 pv_db 与游戏 MOD 目录。</p>
        </ConfigPanel>

        <ConfigPanel title="OBS 覆盖层">
          <Toggle label="启用 OBS 覆盖层" checked={config.obs_overlay_enabled} onChange={(value) => update('obs_overlay_enabled', value)} />
          <Field label="OBS 地址"><input value={config.obs_overlay_host} onChange={(event) => update('obs_overlay_host', event.target.value)} /></Field>
          <Field label="OBS 端口"><input value={config.obs_overlay_port} type="number" onChange={(event) => update('obs_overlay_port', Number(event.target.value))} /></Field>
          <Field label="OBS 标题"><input value={config.obs_overlay_title} onChange={(event) => update('obs_overlay_title', event.target.value)} /></Field>
          <div className="inline-actions">
            <button type="button" className="secondary-button" onClick={startObs}>启动 OBS 覆盖层</button>
            <button type="button" className="ghost-button" onClick={stopObs}>停止</button>
          </div>
          <p className="notice-text">{obsStatus ? `${obsStatus.message} · ${obsStatus.url}` : 'OBS 覆盖层状态读取中…'}</p>
        </ConfigPanel>

        <ConfigPanel title="LLM 与高级设置">
          <Toggle label="启用 LLM 意图识别" checked={config.llm_enabled} onChange={(value) => update('llm_enabled', value)} />
          <p className="notice-text">LLM 为可选功能：启用后，非前缀弹幕会走 OpenAI 兼容接口；本地模型可留空 API Key，云端服务通常需要填写。</p>
          <Field label="API Key（可选）"><input value={config.llm_api_key} type="password" placeholder="本地模型通常可留空" onChange={(event) => update('llm_api_key', event.target.value)} /></Field>
          <Field label="Base URL"><input value={config.llm_base_url} placeholder="例如 https://api.deepseek.com 或 http://127.0.0.1:11434" onChange={(event) => update('llm_base_url', event.target.value)} /></Field>
          <p className="notice-text">Base URL 示例：DeepSeek https://api.deepseek.com；OpenAI https://api.openai.com；Ollama OpenAI 兼容地址 http://127.0.0.1:11434。</p>
          <Field label="模型"><input value={config.llm_model} onChange={(event) => update('llm_model', event.target.value)} /></Field>
        </ConfigPanel>
      </div>

      {rebuildReport ? (
        <div className="panel report-panel">
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
      {toast ? <div className="toast-bubble" role="status">{toast}</div> : null}
    </section>
  );
}

function ConfigPanel({ title, children }: { title: string; children: React.ReactNode }) {
  return <div className="panel config-panel"><h2>{title}</h2>{children}</div>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <label className="field"><span>{label}</span>{children}</label>;
}

function parseRoomId(value: string) {
  const digits = value.replace(/\D/g, '');
  return digits ? Number(digits) : 0;
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (value: boolean) => void }) {
  return <label className="toggle-row"><span>{label}</span><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /></label>;
}
