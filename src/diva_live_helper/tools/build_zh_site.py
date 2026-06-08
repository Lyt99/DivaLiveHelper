"""
build_zh_site — 生成中文曲名数据库静态站点。

用法：
    uv run build-zh-site
    uv run build-zh-site --output-dir docs --repo-url https://github.com/OWNER/REPO
"""

from __future__ import annotations

import argparse
import io
import json
import shutil
import subprocess
import sys
from collections.abc import Mapping
from datetime import UTC, datetime
from pathlib import Path
from typing import cast


INDEX_HTML = """<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>DIVA 中文曲名数据库</title>
  <link rel="icon" href="favicon.svg" type="image/svg+xml" />
  <link rel="stylesheet" href="styles.css" />
</head>
<body>
  <div class="shell">
    <header class="hero">
      <div>
        <p class="eyebrow">DIVA LIVE HELPER / 中文曲名审计台</p>
        <h1>中文曲名数据库</h1>
        <p class="subtitle">浏览、筛选、下载并参与维护 Project DIVA 曲名中文化数据。</p>
      </div>
      <nav class="actions" aria-label="站点操作">
        <a class="button ghost" href="data/song_name_zh.json" download>下载中文名库</a>
        <a class="button ghost" href="data/song_db.json" download>下载歌曲库</a>
        <a class="button ghost" href="data/song_name_zh.audit.tsv" download>下载审计 TSV</a>
        <button class="button" id="exportCsv" type="button">导出当前 CSV</button>
      </nav>
    </header>

    <section class="stats" id="stats" aria-label="数据库统计"></section>

    <section class="panel controls" aria-label="筛选器">
      <label>
        <span>搜索</span>
        <input id="query" type="search" placeholder="原名 / 中文名 / 英文名 / 作者 / 别名" autocomplete="off" />
      </label>
      <label>
        <span>MOD / 来源</span>
        <select id="sourceFilter"></select>
      </label>
      <label>
        <span>作者</span>
        <input id="authorFilter" list="authorOptions" placeholder="输入作者名" autocomplete="off" />
        <datalist id="authorOptions"></datalist>
      </label>
      <label>
        <span>状态</span>
        <select id="statusFilter"></select>
      </label>
      <label>
        <span>证据</span>
        <select id="evidenceFilter"></select>
      </label>
      <label>
        <span>排序</span>
        <select id="sortKey">
          <option value="pv_id">PV ID</option>
          <option value="name">原名</option>
          <option value="name_zh">中文名</option>
          <option value="author">作者</option>
          <option value="source">来源</option>
        </select>
      </label>
      <button id="resetFilters" class="button quiet" type="button">重置筛选</button>
    </section>

    <main class="panel table-panel">
      <div class="table-head">
        <div>
          <strong id="resultCount">加载中...</strong>
          <span id="generatedAt"></span>
        </div>
        <span class="toast" id="toast" aria-live="polite"></span>
        <div class="legend">
          <span><i class="dot translated"></i>已有中文名</span>
          <span><i class="dot missing"></i>待补充</span>
        </div>
      </div>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>PV</th>
              <th>曲名</th>
              <th>中文名</th>
              <th>英文名</th>
              <th>作者</th>
              <th>来源</th>
              <th>证据</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody id="songRows"></tbody>
        </table>
      </div>
    </main>
  </div>

  <dialog id="detailDialog">
    <article class="detail-card">
      <button class="close" id="closeDetail" type="button" aria-label="关闭">×</button>
      <div id="detailContent"></div>
    </article>
  </dialog>

  <script src="app.js"></script>
</body>
</html>
"""


STYLES_CSS = """:root {
  color-scheme: dark;
  --bg: #090b13;
  --panel: rgba(18, 22, 35, 0.82);
  --panel-strong: rgba(29, 36, 56, 0.94);
  --line: rgba(126, 240, 255, 0.18);
  --text: #eef7ff;
  --muted: #8fa3b8;
  --cyan: #6cf7ff;
  --pink: #ff6ab7;
  --gold: #ffe08a;
  --green: #6dffb8;
  --red: #ff7d7d;
  --shadow: 0 24px 80px rgba(0, 0, 0, 0.45);
}

* { box-sizing: border-box; }

body {
  margin: 0;
  min-height: 100vh;
  color: var(--text);
  font-family: "Segoe UI", "Microsoft YaHei", sans-serif;
  background:
    radial-gradient(circle at 10% 0%, rgba(108, 247, 255, 0.18), transparent 34rem),
    radial-gradient(circle at 88% 8%, rgba(255, 106, 183, 0.2), transparent 32rem),
    linear-gradient(140deg, #090b13 0%, #0d1324 45%, #130c19 100%);
}

body::before {
  content: "";
  position: fixed;
  inset: 0;
  pointer-events: none;
  background-image: linear-gradient(rgba(255,255,255,.035) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.035) 1px, transparent 1px);
  background-size: 44px 44px;
  mask-image: linear-gradient(to bottom, rgba(0,0,0,.9), transparent 90%);
}

.shell { width: min(1480px, calc(100vw - 32px)); margin: 0 auto; padding: 36px 0 60px; position: relative; }
.hero { display: flex; align-items: end; justify-content: space-between; gap: 24px; margin-bottom: 24px; }
.eyebrow { margin: 0 0 10px; color: var(--cyan); letter-spacing: .18em; font-size: 12px; font-weight: 800; }
h1 { margin: 0; font-size: clamp(40px, 6vw, 92px); line-height: .9; letter-spacing: -.06em; }
.subtitle { margin: 18px 0 0; color: var(--muted); font-size: 17px; }
.actions { display: flex; flex-wrap: wrap; gap: 10px; justify-content: flex-end; }
.button { border: 1px solid rgba(108,247,255,.35); background: linear-gradient(135deg, rgba(108,247,255,.22), rgba(255,106,183,.2)); color: var(--text); border-radius: 999px; padding: 10px 14px; text-decoration: none; cursor: pointer; font-weight: 700; box-shadow: 0 0 24px rgba(108,247,255,.08); }
.button:hover { transform: translateY(-1px); border-color: rgba(108,247,255,.8); }
.ghost, .quiet { background: rgba(255,255,255,.045); }
.panel, .stat-card { border: 1px solid var(--line); background: var(--panel); backdrop-filter: blur(18px); border-radius: 24px; box-shadow: var(--shadow); }
.stats { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 14px; margin-bottom: 16px; }
.stat-card { padding: 18px; }
.stat-card .label { color: var(--muted); font-size: 12px; letter-spacing: .1em; text-transform: uppercase; }
.stat-card .value { display: block; margin-top: 6px; font-size: 30px; font-weight: 900; }
.controls { display: grid; grid-template-columns: 2fr 1.2fr 1.2fr 1fr 1.2fr 1fr auto; gap: 12px; padding: 16px; margin-bottom: 16px; align-items: end; }
label span { display: block; margin: 0 0 7px 2px; color: var(--muted); font-size: 12px; }
input, select { width: 100%; border: 1px solid rgba(255,255,255,.12); border-radius: 14px; background: rgba(2,5,12,.8); color: var(--text); padding: 11px 12px; outline: none; }
input:focus, select:focus { border-color: var(--cyan); box-shadow: 0 0 0 3px rgba(108,247,255,.12); }
.table-panel { overflow: hidden; }
.table-head { display: flex; justify-content: space-between; gap: 12px; padding: 18px 20px; border-bottom: 1px solid var(--line); color: var(--muted); }
.legend { display: flex; gap: 16px; }
.toast { min-height: 20px; color: var(--green); font-size: 13px; }
.dot { display: inline-block; width: 9px; height: 9px; border-radius: 50%; margin-right: 6px; }
.translated { background: var(--green); }
.missing { background: var(--red); }
.table-wrap { overflow: auto; max-height: 68vh; }
table { width: 100%; border-collapse: collapse; min-width: 1200px; }
th, td { padding: 13px 14px; border-bottom: 1px solid rgba(255,255,255,.07); text-align: left; vertical-align: top; }
th { position: sticky; top: 0; z-index: 1; background: rgba(13,17,28,.96); color: var(--cyan); font-size: 12px; letter-spacing: .08em; }
tbody tr:hover { background: rgba(108,247,255,.06); }
.song-title { font-weight: 850; }
.muted { color: var(--muted); }
.tag { display: inline-flex; align-items: center; border: 1px solid rgba(255,255,255,.12); border-radius: 999px; padding: 4px 8px; color: var(--text); background: rgba(255,255,255,.055); font-size: 12px; margin: 2px 4px 2px 0; white-space: nowrap; }
.evidence { color: var(--gold); }
.missing-text { color: var(--red); font-style: italic; }
.row-actions { display: flex; gap: 7px; flex-wrap: wrap; }
.small { padding: 7px 10px; font-size: 12px; }
dialog { width: min(760px, calc(100vw - 32px)); border: 1px solid var(--line); border-radius: 26px; padding: 0; background: var(--panel-strong); color: var(--text); box-shadow: var(--shadow); }
dialog::backdrop { background: rgba(0,0,0,.68); backdrop-filter: blur(4px); }
.detail-card { position: relative; padding: 28px; }
.close { position: absolute; top: 16px; right: 16px; width: 36px; height: 36px; border: 1px solid rgba(255,255,255,.16); border-radius: 50%; background: rgba(255,255,255,.06); color: var(--text); font-size: 22px; cursor: pointer; }
pre { white-space: pre-wrap; overflow: auto; background: rgba(0,0,0,.28); border: 1px solid rgba(255,255,255,.1); border-radius: 16px; padding: 14px; }

@media (max-width: 1100px) {
  .hero { align-items: start; flex-direction: column; }
  .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .controls { grid-template-columns: 1fr 1fr; }
}

@media (max-width: 650px) {
  .stats, .controls { grid-template-columns: 1fr; }
  .table-head { flex-direction: column; }
}
"""


APP_JS = r"""const state = {
  rows: [],
  manifest: {},
  filters: { query: '', source: '', author: '', status: '', evidence: '', sortKey: 'pv_id' },
};

const $ = (id) => document.getElementById(id);
const esc = (value) => String(value ?? '').replace(/[&<>"]/g, (ch) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[ch]));
const norm = (value) => String(value ?? '').toLowerCase();

const evidenceLabels = {
  'jiut-song-name-table': 'jiut 中文歌词 Mod 对照表',
  'netease-transname': '网易云 transNames',
  'netease-alias': '网易云别名',
  'qq-music-parenthetical-title': 'QQ 音乐括号标题',
  'bilibili-title-evidence': 'B站标题证据',
  'moegirl-title-evidence': '萌娘百科标题证据',
  'community-title-evidence': '社区标题证据',
  't-wy-pjsk-alias': 'PJSK 社区别名',
  'pjsk-tc-master': 'PJSK 繁中库',
  'pjsk-tc-master-normalized': 'PJSK 繁中规范化',
  'pdaft-zh-release-pv-db': 'PDAFT 中文补丁',
  'self-display-fallback': '原题自显示',
  'self-display-cjk-title': '汉字标题自显示',
  'self-display-latin-title': '英文标题自显示',
  'self-display-kana-title': '假名标题自显示',
  'name-normalized+existing-normalized': '规范化匹配',
  'name-normalized+existing-exact': '精确规范化匹配',
};

async function loadData() {
  const [songDb, zhDb, manifest] = await Promise.all([
    fetch('data/song_db.json').then((r) => r.json()),
    fetch('data/song_name_zh.json').then((r) => r.json()),
    fetch('data/manifest.json').then((r) => r.json()),
  ]);
  state.manifest = manifest;
  const zhEntries = zhDb.entries || {};
  state.rows = Object.entries(songDb.songs || {}).map(([pvId, song]) => {
    const zh = zhEntries[song.name] || null;
    const authors = Array.isArray(song.authors) ? song.authors : [];
    return {
      pv_id: Number(pvId),
      name: song.name || '',
      name_en: zh?.name_en || song.name_en || '',
      name_zh: zh?.name_zh || '',
      status: zh?.status || 'pending',
      source: song.source || zh?.source || '',
      zh_source: zh?.source || '',
      author: zh?.author || authors.join(', '),
      authors,
      aliases: Array.isArray(song.aliases) ? song.aliases : [],
      evidence: zh?.evidence || 'missing',
      candidate: zh?.candidate || '',
      difficulty: song.difficulty || {},
      has_zh: Boolean(zh?.name_zh),
    };
  });
}

function fillSelect(id, values, labelAll) {
  const select = $(id);
  select.innerHTML = `<option value="">${esc(labelAll)}</option>` + values.map((value) => `<option value="${esc(value)}">${esc(evidenceLabels[value] || value)}</option>`).join('');
}

function initControls() {
  fillSelect('sourceFilter', state.manifest.sources || [], '全部来源');
  fillSelect('statusFilter', state.manifest.statuses || [], '全部状态');
  fillSelect('evidenceFilter', state.manifest.evidence || [], '全部证据');
  $('authorOptions').innerHTML = (state.manifest.authors || []).map((author) => `<option value="${esc(author)}"></option>`).join('');
  for (const id of ['query', 'sourceFilter', 'authorFilter', 'statusFilter', 'evidenceFilter', 'sortKey']) {
    $(id).addEventListener('input', () => {
      state.filters.query = $('query').value.trim();
      state.filters.source = $('sourceFilter').value;
      state.filters.author = $('authorFilter').value.trim();
      state.filters.status = $('statusFilter').value;
      state.filters.evidence = $('evidenceFilter').value;
      state.filters.sortKey = $('sortKey').value;
      render();
    });
  }
  $('resetFilters').addEventListener('click', () => {
    for (const id of ['query', 'sourceFilter', 'authorFilter', 'statusFilter', 'evidenceFilter']) $(id).value = '';
    $('sortKey').value = 'pv_id';
    state.filters = { query: '', source: '', author: '', status: '', evidence: '', sortKey: 'pv_id' };
    render();
  });
  $('exportCsv').addEventListener('click', exportCsv);
  $('closeDetail').addEventListener('click', () => $('detailDialog').close());
}

function filteredRows() {
  const query = norm(state.filters.query);
  const author = norm(state.filters.author);
  return state.rows.filter((row) => {
    const text = norm([row.name, row.name_zh, row.name_en, row.author, row.aliases.join(' ')].join(' '));
    if (query && !text.includes(query)) return false;
    if (state.filters.source && row.source !== state.filters.source && row.zh_source !== state.filters.source) return false;
    if (author && !norm(row.author).includes(author)) return false;
    if (state.filters.status && row.status !== state.filters.status) return false;
    if (state.filters.evidence && row.evidence !== state.filters.evidence) return false;
    return true;
  }).sort((a, b) => {
    const key = state.filters.sortKey;
    if (key === 'pv_id') return a.pv_id - b.pv_id;
    return String(a[key] || '').localeCompare(String(b[key] || ''), 'zh-CN');
  });
}

function renderStats() {
  const translated = state.rows.filter((row) => row.has_zh).length;
  const mods = new Set(state.rows.map((row) => row.source).filter((source) => source.startsWith('mod:'))).size;
  const evidence = new Set(state.rows.map((row) => row.evidence)).size;
  const cards = [
    ['总曲数', state.rows.length],
    ['中文名', translated],
    ['覆盖率', `${((translated / state.rows.length) * 100).toFixed(1)}%`],
    ['MOD 来源', mods],
    ['证据类型', evidence],
  ];
  $('stats').innerHTML = cards.map(([label, value]) => `<div class="stat-card"><span class="label">${esc(label)}</span><span class="value">${esc(value)}</span></div>`).join('');
  $('generatedAt').textContent = state.manifest.generated_at ? `生成于 ${state.manifest.generated_at}` : '';
}

function render() {
  const rows = filteredRows();
  $('resultCount').textContent = `显示 ${rows.length} / ${state.rows.length} 首`;
  $('songRows').innerHTML = rows.map(rowHtml).join('');
  for (const button of document.querySelectorAll('[data-detail]')) button.addEventListener('click', () => showDetail(Number(button.dataset.detail)));
  for (const button of document.querySelectorAll('[data-copy]')) button.addEventListener('click', () => copyEntry(Number(button.dataset.copy)));
}

function rowHtml(row) {
  const zh = row.name_zh ? esc(row.name_zh) : '<span class="missing-text">待补充</span>';
  const issueUrl = buildIssueUrl(row);
  return `<tr>
    <td><span class="tag">${row.pv_id}</span></td>
    <td><div class="song-title">${esc(row.name)}</div>${row.aliases.length ? `<div class="muted">别名：${esc(row.aliases.slice(0, 4).join(' / '))}</div>` : ''}</td>
    <td>${zh}</td>
    <td class="muted">${esc(row.name_en)}</td>
    <td>${esc(row.author)}</td>
    <td><span class="tag">${esc(row.source)}</span>${row.zh_source && row.zh_source !== row.source ? `<span class="tag">译名:${esc(row.zh_source)}</span>` : ''}</td>
    <td><span class="tag evidence">${esc(evidenceLabels[row.evidence] || row.evidence)}</span></td>
    <td><div class="row-actions"><button class="button quiet small" data-detail="${row.pv_id}" type="button">详情</button><button class="button quiet small" data-copy="${row.pv_id}" type="button">复制</button>${issueUrl ? `<a class="button quiet small" href="${esc(issueUrl)}" target="_blank" rel="noreferrer">建议修改</a>` : ''}</div></td>
  </tr>`;
}

function showDetail(pvId) {
  const row = state.rows.find((item) => item.pv_id === pvId);
  if (!row) return;
  const entry = {
    [row.name]: {
      name_zh: row.name_zh,
      status: row.status,
      source: row.zh_source || row.source,
      name_en: row.name_en,
      author: row.author,
      candidate: row.candidate,
      evidence: row.evidence,
    },
  };
  $('detailContent').innerHTML = `<p class="eyebrow">PV ${row.pv_id}</p><h2>${esc(row.name_zh || row.name)}</h2>
    <p class="muted">${esc(row.name)} / ${esc(row.name_en)} / ${esc(row.author)}</p>
    <p><span class="tag">${esc(row.source)}</span><span class="tag evidence">${esc(evidenceLabels[row.evidence] || row.evidence)}</span><span class="tag">${esc(row.status)}</span></p>
    <h3>JSON 片段</h3><pre>${esc(JSON.stringify(entry, null, 2))}</pre>`;
  $('detailDialog').showModal();
}

async function copyEntry(pvId) {
  const row = state.rows.find((item) => item.pv_id === pvId);
  if (!row) return;
  const text = `${row.name}\t${row.name_zh}\t${row.evidence}`;
  try {
    await navigator.clipboard.writeText(text);
    showToast('已复制曲目片段');
  } catch (error) {
    console.error(error);
    showToast('复制失败，请手动打开详情复制');
  }
}

function buildIssueUrl(row) {
  if (!state.manifest.repo_url) return '';
  const body = [
    '### 曲目信息',
    `- PV: ${row.pv_id}`,
    `- 原名: ${row.name}`,
    `- 当前中文名: ${row.name_zh || '(待补充)'}`,
    `- 英文名: ${row.name_en}`,
    `- 作者: ${row.author}`,
    `- 来源: ${row.source}`,
    '',
    '### 建议',
    '- 建议中文名：',
    '- 证据链接：',
    '- 证据说明：',
  ].join('\n');
  return `${state.manifest.repo_url}/issues/new?title=${encodeURIComponent(`中文名建议: ${row.name}`)}&body=${encodeURIComponent(body)}`;
}

function exportCsv() {
  const rows = filteredRows();
  const header = ['pv_id','name','name_zh','name_en','author','source','status','evidence'];
  const csv = '\ufeff' + [header.join(',')].concat(rows.map((row) => header.map((key) => csvCell(row[key])).join(','))).join('\r\n');
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'diva_zh_song_names.csv';
  a.click();
  URL.revokeObjectURL(url);
}

function csvCell(value) {
  let text = String(value ?? '');
  if (/^[=+\-@\t\r]/.test(text)) text = `'${text}`;
  return `"${text.replace(/"/g, '""')}"`;
}

function showToast(message) {
  const toast = $('toast');
  toast.textContent = message;
  clearTimeout(showToast.timer);
  showToast.timer = setTimeout(() => { toast.textContent = ''; }, 1800);
}

loadData().then(() => {
  initControls();
  renderStats();
  render();
}).catch((error) => {
  $('resultCount').textContent = `加载失败：${error}`;
});
"""


FAVICON_SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <defs>
    <linearGradient id="g" x1="0" x2="1" y1="0" y2="1">
      <stop stop-color="#6cf7ff"/>
      <stop offset="1" stop-color="#ff6ab7"/>
    </linearGradient>
  </defs>
  <rect width="64" height="64" rx="16" fill="#090b13"/>
  <path d="M18 45V17h8l6 15 6-15h8v28h-7V29l-5 12h-4l-5-12v16z" fill="url(#g)"/>
</svg>
"""


def _configure_console_encoding():
    if isinstance(sys.stdout, io.TextIOWrapper):
        sys.stdout.reconfigure(encoding="utf-8")
    if isinstance(sys.stderr, io.TextIOWrapper):
        sys.stderr.reconfigure(encoding="utf-8")


def _detect_repo_url() -> str:
    try:
        result = subprocess.run(
            ["git", "config", "--get", "remote.origin.url"],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
    except OSError:
        return ""
    remote = result.stdout.strip()
    if not remote:
        return ""
    if remote.startswith("git@github.com:"):
        remote = "https://github.com/" + remote.removeprefix("git@github.com:")
    if remote.endswith(".git"):
        remote = remote[:-4]
    return remote


def _normalize_repo_url(repo_url: str) -> str:
    if not repo_url:
        return ""
    normalized = repo_url.rstrip("/")
    if normalized.startswith("https://github.com/") or normalized.startswith("https://gitlab.com/"):
        return normalized
    print(f"警告: repo URL 不是受支持的 HTTPS Git 托管地址，已忽略: {repo_url}")
    return ""


def _copy_file(src: Path, dst: Path):
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(src, dst)


def _load_json(path: Path) -> dict[str, object]:
    with open(path, "r", encoding="utf-8") as f:
        raw: object = json.load(f)
    if not isinstance(raw, dict):
        raise ValueError(f"{path} 不是 JSON object")
    return cast(dict[str, object], raw)


def _as_mapping(value: object) -> Mapping[str, object] | None:
    if isinstance(value, Mapping):
        return cast(Mapping[str, object], value)
    return None


def _string_list(value: object) -> list[str]:
    if not isinstance(value, list):
        return []
    return [str(item) for item in value if item]


def _build_manifest(data_dir: Path, repo_url: str) -> dict[str, object]:
    song_db = _load_json(data_dir / "song_db.json")
    zh_db = _load_json(data_dir / "song_name_zh.json")
    songs = song_db.get("songs")
    entries = zh_db.get("entries")
    if song_db.get("version") != 1 or not isinstance(songs, dict):
        raise ValueError("song_db.json 必须是 version=1 且包含 songs")
    if zh_db.get("version") != 3 or not isinstance(entries, dict):
        raise ValueError("song_name_zh.json 必须是 version=3 且包含 entries")
    song_items = cast(dict[str, object], songs)
    entry_items = cast(dict[str, object], entries)

    sources: set[str] = set()
    authors: set[str] = set()
    statuses: set[str] = set()
    evidence: set[str] = set()
    mod_total = 0
    mod_covered = 0
    for song_raw in song_items.values():
        song = _as_mapping(song_raw)
        if song is None:
            continue
        source = str(song.get("source") or "")
        if source:
            sources.add(source)
        if source.startswith("mod:"):
            mod_total += 1
            if song.get("name") in entry_items:
                mod_covered += 1
        authors.update(_string_list(song.get("authors")))
    for entry_raw in entry_items.values():
        entry = _as_mapping(entry_raw)
        if entry is None:
            continue
        source = str(entry.get("source") or "")
        if source:
            sources.add(source)
        status = str(entry.get("status") or "")
        if status:
            statuses.add(status)
        item_evidence = str(entry.get("evidence") or "")
        if item_evidence:
            evidence.add(item_evidence)
        author = str(entry.get("author") or "")
        if author:
            for part in author.split(","):
                if part.strip():
                    authors.add(part.strip())

    return {
        "generated_at": datetime.now(UTC).astimezone().isoformat(timespec="seconds"),
        "repo_url": repo_url,
        "total_songs": len(song_items),
        "total_zh_names": len(entry_items),
        "mod_total": mod_total,
        "mod_covered": mod_covered,
        "sources": sorted(sources),
        "authors": sorted(authors),
        "statuses": sorted(statuses),
        "evidence": sorted(evidence),
    }


def main():
    _configure_console_encoding()
    parser = argparse.ArgumentParser(
        prog="build-zh-site",
        description="生成可发布的中文曲名数据库静态站点",
    )
    parser.add_argument("--config", metavar="PATH", default="config.json", help="配置文件路径（默认 config.json）")
    parser.add_argument("--data-dir", metavar="DIR", default=None, help="数据文件目录（覆盖 config.json 中的 data_dir）")
    parser.add_argument("--output-dir", metavar="DIR", default="docs", help="静态站点输出目录（默认 docs）")
    parser.add_argument("--repo-url", metavar="URL", default=None, help="GitHub 仓库 URL，用于贡献链接")
    args = parser.parse_args()

    from diva_live_helper.config import Config

    config = Config()
    config.load(args.config)
    data_dir = Path(args.data_dir or config.data_dir)
    output_dir = Path(args.output_dir)
    repo_url = _normalize_repo_url(args.repo_url if args.repo_url is not None else _detect_repo_url())

    required = ["song_db.json", "song_name_zh.json", "song_name_zh.audit.tsv"]
    missing = [name for name in required if not (data_dir / name).exists()]
    if missing:
        print(f"错误: 数据文件缺失: {', '.join(missing)}", file=sys.stderr)
        sys.exit(1)

    manifest = _build_manifest(data_dir, repo_url)

    output_dir.mkdir(parents=True, exist_ok=True)
    data_output = output_dir / "data"
    data_output.mkdir(parents=True, exist_ok=True)
    for filename in required:
        _copy_file(data_dir / filename, data_output / filename)
    (data_output / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (output_dir / "index.html").write_text(INDEX_HTML, encoding="utf-8")
    (output_dir / "styles.css").write_text(STYLES_CSS, encoding="utf-8")
    (output_dir / "app.js").write_text(APP_JS, encoding="utf-8")
    (output_dir / "favicon.svg").write_text(FAVICON_SVG, encoding="utf-8")
    (output_dir / "favicon.ico").write_bytes(b"")

    print("=== build-zh-site ===")
    print(f"数据目录  : {data_dir}")
    print(f"输出目录  : {output_dir}")
    print(f"歌曲总数  : {manifest['total_songs']}")
    print(f"中文名数  : {manifest['total_zh_names']}")
    print(f"MOD覆盖率 : {manifest['mod_covered']} / {manifest['mod_total']}")
    if repo_url:
        print(f"贡献链接  : {repo_url}/issues/new")
    else:
        print("贡献链接  : 未配置 repo URL（页面仍可浏览/下载）")


if __name__ == "__main__":
    main()
