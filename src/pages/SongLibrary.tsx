import { useEffect, useMemo, useState } from 'react';
import { api } from '../lib/tauri';
import type { SongInfo } from '../types';

export default function LibraryPage() {
  const [songs, setSongs] = useState<SongInfo[]>([]);
  const [query, setQuery] = useState('');
  const [message, setMessage] = useState('');

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

  async function jump(songId: number) {
    try {
      const result = await api.changeSong(songId);
      setMessage(result);
    } catch (error) {
      setMessage(String(error));
    }
  }

  return (
    <section className="page">
      <header className="page-header split">
        <div><p className="eyebrow">Song Database</p><h1>歌曲库</h1><p className="muted">共 {songs.length} 首，列表最多显示 300 条匹配结果。</p></div>
        <input className="search-input" placeholder="搜索日文名 / 中文名 / 英文名 / 作者 / 别名" value={query} onChange={(event) => setQuery(event.target.value)} />
      </header>
      <div className="panel table-panel">
        <div className="song-table">
          {filtered.map((song) => (
            <div key={song.pv_id} className="song-row">
              <span className="mono">#{song.pv_id}</span>
              <div><strong>{song.name_zh || song.name}</strong><span>{song.name}{song.name_en ? ` · ${song.name_en}` : ''}</span></div>
              <span>{song.authors[0] || '未知作者'}</span>
              <span>{formatDifficulty(song.difficulty)}</span>
              <button type="button" className="ghost-button" onClick={() => jump(song.pv_id)}>立即切歌</button>
            </div>
          ))}
        </div>
      </div>
      <div className="message-bar">{message}</div>
    </section>
  );
}

function formatDifficulty(difficulty: Record<string, number>) {
  return ['easy', 'normal', 'hard', 'extreme', 'exextreme'].filter((key) => difficulty[key]).map((key) => `${key}:${difficulty[key]}`).join(' / ') || '无难度信息';
}
