# AGENTS.md

## Project Overview

B站直播点歌助手桌面版 — Rust + Tauri + React app that listens to Bilibili live room danmaku, parses song requests, manages a request queue, and switches songs in *Hatsune Miku Project DIVA Mega Mix Plus* by writing to game memory.

**Windows-only.** Memory injection targets `DivaMegaMix.exe` from `src-tauri/src/song_select.rs`. Global hotkey registration may require administrator privileges.

## Developer Commands

```bash
# Install frontend / Tauri CLI dependencies
npm install

# Run desktop app in development
npm run tauri dev

# Build desktop app
npm run tauri build

# Frontend typecheck + Vite build
npm run build

# Rust backend check
cargo check --manifest-path src-tauri/Cargo.toml
```

No dedicated test suite exists yet.

## Package Layout

```text
src/                         # React / TypeScript frontend
  App.tsx                    # Routes and event listeners
  main.tsx                   # React entry point
  index.css                  # Desktop UI styling
  types.ts                   # Frontend API/event types
  lib/tauri.ts               # invoke() wrappers and emptyConfig
  components/                # Layout, theme toggle, shared UI
  pages/                     # Queue, Config, Logs, SongLibrary pages

src-tauri/                   # Rust / Tauri backend
  Cargo.toml                 # Rust manifest
  tauri.conf.json            # Tauri v2 config
  capabilities/default.json  # Tauri permissions
  icons/icon.ico             # App icon
  src/
    main.rs                  # Tauri binary entry
    lib.rs                   # Builder setup, state, command registration
    commands.rs              # Tauri commands exposed to frontend
    config.rs                # Config model, load/save/validation
    danmaku.rs               # Bilibili danmaku websocket client
    db_tool.rs               # Song DB rebuild helpers
    hotkey.rs                # Global shortcut integration
    llm_intent.rs            # OpenAI-compatible intent parser
    obs_overlay.rs           # Local OBS browser-source server
    queue.rs                 # Song queue and history
    song_db.rs               # Song DB models and parsing
    song_search.rs           # Multi-pass song search
    song_select.rs           # Game memory writes
```

The previous Python app has been removed. Do not reintroduce `pyproject.toml`, `uv.lock`, `.python-version`, `fast_jump_en.py`, or `src/diva_live_helper/` unless explicitly requested.

## Required Config (`config.json`)

Copy from `config.example.json` or save once from the desktop Settings page. `config.json` is ignored because it may contain SESSDATA and LLM API keys.

Key non-obvious fields:
- `mods_dir` — absolute path to game's `mods/` folder; each mod must have `rom/mod_pv_db.txt`
- `sessdata` — Bilibili SESSDATA cookie; improves danmaku token flow reliability
- `http_proxy` — optional proxy for the LLM client
- `llm_enabled` — enables semantic danmaku parsing; prefix requests still use local parsing first
- `llm_api_key` — optional so local OpenAI-compatible models can run without a key
- `llm_base_url` — OpenAI-compatible; default is DeepSeek
- `default_search_difficulty` — difficulty tier used for star-rating filter (`"extreme"` by default); options: `"easy"` / `"normal"` / `"hard"` / `"extreme"` / `"exextreme"`
- `difficulty_tolerance` — ±tolerance for star-rating filter (default `0.5`)

## Data Files (`Data/`)

Files required at startup:
- `song_db.json` — primary structured song DB; keyed by `pv_id` string; contains name/name_en/aliases/authors/difficulty/source; does **not** contain Chinese names
- `song_name_zh.json` — independent Chinese name DB; version 3 stores one `entries` object keyed by original song name
- `AnotherSongName.json` — legacy alias map `{ alias: canonical_name }`; imported into search aliases
- `HanziKanjiDict.txt` — hanzi→kanji conversion for cross-script search

Files used when rebuilding the local DB:
- `base_song_db.json` — embedded official song DB; keyed by `pv_id` string; includes base-game and DLC songs, original name, English name, Chinese name, authors, difficulty, empty aliases, and `source: "base"` or `source: "dlc"`
- `pv_db.txt` / `mdata_pv_db.txt` — legacy base-game and DLC fallbacks used only when `base_song_db.json` is missing

Generated public site:
- `docs/` — static GitHub Pages-compatible Chinese-name database browser containing frontend assets and `docs/data/` copies of public data.

DLC songs are part of `base_song_db.json` and are marked with `source: "dlc"`. MOD songs are not part of the embedded base DB; they come from mod packs under `mods_dir`.

## Architecture Notes

### Danmaku → Song Request Flow
1. Frontend starts danmaku via `start_danmaku(roomId)` Tauri command.
2. Rust resolves the real room id, fetches WBI keys, signs `getDanmuInfo`, connects to the returned websocket host, and emits `danmaku` / `connection-status` events.
3. If prefix matches (`点歌 <name>`) → parse immediately, optionally strip trailing difficulty keyword (e.g. `ex`, `7星`), skip LLM.
4. Otherwise, if LLM is enabled → call OpenAI-compatible chat completions and extract `song_name`, `author`, `difficulty`.
5. `SongSearcher` runs multi-pass lookup with optional star-rating filter.
6. First result is added to `SongQueue`; duplicates are rejected by default.
7. Hotkey or UI action dequeues next song and calls `SongSelector::change_song(pv_id)`.

### Search Pass Order
1. Japanese/original name (fuzzy)
2. Chinese name (fuzzy, from `song_name_zh.json`)
3. Hanzi→kanji conversion → Japanese name
4. Alias index (`song_db.json` aliases + `AnotherSongName.json`)
5. English name (fuzzy)

### Memory Layout (`song_select.rs`)
Game process: `DivaMegaMix.exe`. Key offsets from `base_address`:
- `0x12B6350` — `LastSelectPVIDMem` (song ID)
- `0x12B6354` — `LastSelectSortMem` (sort mode; set to `1` = difficulty)
- `0x12B635C` — `LastSelectDiffMem` (difficulty; set to `19` = ALL)
- `0xCC61098` — `ChangeSongSelect` (set to `5` to trigger switch)
- `0xCC610A0` — `StartChange` (set to `2`)

**Eden offset quirk**: if `LastSelectPVIDMem` reads `0`, adds `0x105F460` to the select addresses. This handles the "Eden" game variant.

## Conventions

- Root directory is the Tauri project root; do not put the active Rust project under `app/`.
- All user-facing strings and comments are in Chinese.
- Keep `config.json` ignored; never commit credentials/cookies.
- Tauri resource path for bundled data is `src-tauri/tauri.conf.json` → `bundle.resources: ["../Data"]`.
- VocaDB does **not** reliably provide Chinese song names; do not rely on it for bulk Chinese name population.
