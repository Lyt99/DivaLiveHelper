import { useEffect, useMemo, useState } from 'react';
import { api } from '../lib/tauri';
import type { SongInfo } from '../types';

type DifficultyTier = 'easy' | 'normal' | 'hard' | 'extreme' | 'exextreme';

const DIFFICULTY_BUTTONS: { key: DifficultyTier; label: string }[] = [
  { key: 'easy', label: '简单' },
  { key: 'normal', label: '普通' },
  { key: 'hard', label: '困难' },
  { key: 'extreme', label: '极限' },
  { key: 'exextreme', label: 'EX极限' },
];

export default function LibraryPage() {
  const [songs, setSongs] = useState<SongInfo[]>([]);
  const [query, setQuery] = useState('');
  const [message, setMessage] = useState('');
  const [switchingKey, setSwitchingKey] = useState<string | null>(null);

  useEffect(() => {
    api.getAllSongs().then(setSongs).catch((error) => setMessage(String(error)));
  }, []);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return songs.slice(0, 300);
    return songs
      .filter((song) => [song.name, song.name_en, song.name_zh, ...song.authors, ...song.aliases].filter(Boolean).some((value) => String(value).toLowerCase().includes(needle)))
      .slice(0, 300);
  }, [query, songs]);

  async function jump(song: SongInfo, tier: DifficultyTier) {
    const actionKey = `${song.pv_id}:${tier}`;
    setSwitchingKey(actionKey);
    try {
      const result = await api.changeSong(song.pv_id, tier);
      const label = DIFFICULTY_BUTTONS.find((item) => item.key === tier)?.label ?? tier;
      setMessage(`${result}（${song.name_zh || song.name} · ${label}）`);
    } catch (error) {
      setMessage(String(error));
    } finally {
      setSwitchingKey(null);
    }
  }

  return (
    <section className="page">
      <header className="page-header split">
        <div><h1>歌曲库</h1><p className="muted">共 {songs.length} 首，列表最多显示 300 条匹配结果。</p></div>
        <input className="search-input" placeholder="搜索日文名 / 中文名 / 英文名 / 作者 / 别名" value={query} onChange={(event) => setQuery(event.target.value)} />
      </header>
      <div className="panel table-panel">
        <div className="song-table">
          {filtered.map((song) => (
            <div key={song.pv_id} className="song-row">
              <span className="mono">#{song.pv_id}</span>
              <div><strong>{song.name_zh || song.name}</strong><span>{song.name}{song.name_en ? ` · ${song.name_en}` : ''}</span></div>
              <span>{song.authors[0] || '未知作者'}</span>
              <DifficultyButtons song={song} switchingKey={switchingKey} onJump={jump} />
            </div>
          ))}
        </div>
      </div>
      <div className="message-bar">{message}</div>
    </section>
  );
}

function DifficultyButtons({
  song,
  switchingKey,
  onJump,
}: {
  song: SongInfo;
  switchingKey: string | null;
  onJump: (song: SongInfo, tier: DifficultyTier) => void;
}) {
  const available = DIFFICULTY_BUTTONS.filter(({ key }) => song.difficulty[key] !== undefined);
  if (available.length === 0) {
    return <span className="difficulty-empty">无难度信息</span>;
  }

  return (
    <div className="library-difficulty-buttons" aria-label={`${song.name_zh || song.name} 可选难度`}>
      {available.map(({ key, label }) => {
        const actionKey = `${song.pv_id}:${key}`;
        const level = song.difficulty[key];
        return (
          <button
            key={key}
            type="button"
            className={`difficulty-jump difficulty-${key}`}
            disabled={switchingKey !== null}
            title={`切换到 ${song.name_zh || song.name} 的${label}难度`}
            onClick={() => onJump(song, key)}
          >
            <span>{switchingKey === actionKey ? '切换中' : label}</span>
            <strong>{level.toFixed(1)}</strong>
          </button>
        );
      })}
    </div>
  );
}
