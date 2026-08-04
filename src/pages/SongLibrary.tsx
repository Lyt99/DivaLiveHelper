import { useEffect, useMemo, useState } from 'react';
import { api } from '../lib/tauri';
import { reportStatus } from '../lib/status';
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
  const [switchingKey, setSwitchingKey] = useState<string | null>(null);

  useEffect(() => {
    api.getAllSongs().then(setSongs).catch((error) => reportStatus(String(error)));
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
      reportStatus(`${result}（${song.name_zh || song.name} · ${label}）`);
    } catch (error) {
      reportStatus(String(error));
    } finally {
      setSwitchingKey(null);
    }
  }

  return (
    <section className="page library-page">
      <header className="toolbar">
        <input className="search-input" placeholder="搜索日文名 / 中文名 / 英文名 / 作者 / 别名" value={query} onChange={(event) => setQuery(event.target.value)} />
        <span className="toolbar-meta">共 {songs.length} 首 · 最多显示 300 条匹配</span>
      </header>
      <div className="song-table-wrap">
        <div className="song-table">
          <div className="song-row song-head" aria-hidden="true">
            <span>ID</span>
            <span>曲名</span>
            <span>作者</span>
            <span>切歌</span>
          </div>
          {filtered.map((song) => (
            <div key={song.pv_id} className="song-row">
              <span className="song-id">#{song.pv_id}</span>
              <div>
                <strong>{song.name_zh || song.name}</strong>
                <span className="song-sub">{song.name}{song.name_en ? ` · ${song.name_en}` : ''}</span>
              </div>
              <span className="song-author">{song.authors[0] || '未知作者'}</span>
              <DifficultyButtons song={song} switchingKey={switchingKey} onJump={jump} />
            </div>
          ))}
        </div>
      </div>
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
        const level = song.difficulty[key];
        return (
          <button
            key={key}
            type="button"
            className={`difficulty-jump difficulty-${key}`}
            disabled={switchingKey !== null}
            title={`切换到 ${song.name_zh || song.name} 的${label}难度（${level.toFixed(1)} 星）`}
            onClick={() => onJump(song, key)}
          >
            <span className="difficulty-jump-label">{label}</span>
            <span className="difficulty-jump-rating">
              <svg width="10" height="10" viewBox="0 0 20 20" fill="currentColor" aria-hidden="true">
                <path d="M10 1 L12.1 7.1 L18.6 7.2 L13.4 11.1 L15.3 17.3 L10 13.6 L4.7 17.3 L6.6 11.1 L1.4 7.2 L7.9 7.1 Z" />
              </svg>
              {level.toFixed(1)}
            </span>
          </button>
        );
      })}
    </div>
  );
}
