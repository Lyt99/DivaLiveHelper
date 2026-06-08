const state = {
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
