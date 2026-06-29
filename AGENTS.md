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

Rust backend unit tests exist and can be run with:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Package Layout

```text
src/                         # React / TypeScript frontend
  App.tsx                    # Routes and event listeners
  main.tsx                   # React entry point
  index.css                  # Desktop UI styling
  types.ts                   # Frontend API/event types
  lib/tauri.ts               # invoke() wrappers and emptyConfig
  components/                # Layout, theme toggle, shared UI
  pages/                     # Queue, Config, Logs, SongLibrary, Wizard (first-run setup) pages

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

Copy from `config.example.json` or save once from the desktop Settings page (or the first-run Wizard). `config.json` is git-ignored because it may contain SESSDATA and LLM API keys. At runtime the backend reads it from the executable's parent directory (`exe_dir/config.json`, see `lib.rs::run`); for `npm run tauri dev` this resolves to the repo root. `config.example.json` is out of date and does not include every field below — missing fields fall back to `Config::default()` in `src-tauri/src/config.rs`.

Key non-obvious fields:
- `room_id` — Bilibili live room ID; `validate()` rejects `0`
- `hotkey` — global shortcut for switching songs, e.g. `ctrl+shift+n`
- `data_dir` — runtime data directory, default `Data`; resolved against the exe dir / bundled resource dir by `resolve_data_dir` in `lib.rs`
- `mods_dir` — absolute path to game's `mods/` folder; each mod must have `rom/mod_pv_db.txt`
- `auto_play_next` / `auto_play_interval` — when enabled, auto-dequeue the next song after N seconds
- `max_queue_size` / `allow_duplicates` — queue capacity and duplicate policy
- `obs_overlay_enabled` / `obs_overlay_host` / `obs_overlay_port` / `obs_overlay_title` — local OBS browser-source server; `validate()` requires the host to be `127.0.0.1` or `localhost`
- `song_command_prefix` — danmaku prefix that triggers local parsing, default `点歌`
- `sessdata` — Bilibili SESSDATA cookie; improves danmaku token flow reliability
- `fetch_chinese_names` — toggle fetching Chinese names from external sources
- `http_proxy` — optional proxy for the LLM client
- `llm_enabled` — enables semantic danmaku parsing; prefix requests still use local parsing first
- `llm_api_key` — optional so local OpenAI-compatible models can run without a key
- `llm_base_url` — OpenAI-compatible; default is DeepSeek
- `llm_model` — model name, default `deepseek-chat`
- `llm_max_tokens` — response token cap, default `150`
- `default_search_difficulty` — difficulty tier used for star-rating filter (`"extreme"` by default); options: `"easy"` / `"normal"` / `"hard"` / `"extreme"` / `"exextreme"`
- `difficulty_tolerance` — ±tolerance for star-rating filter (default `0.5`)
- `difficulty_fallback` — direction when no song matches the target tier within tolerance: `"easier"` (default) or `"harder"`

## Data Files (`Data/`)

Files read at startup:
- `song_db.json` — primary structured song DB; keyed by `pv_id` string; contains name/name_en/aliases/authors/difficulty/source; does **not** persist Chinese names (they are merged at runtime). Bundled as a Tauri resource.
- `song_name_zh.json` — Chinese name DB; version 3 stores one `entries` object keyed by original song name. Embedded into the Rust binary via `include_str!` as the base; the on-disk copy (if present in `data_dir`) overrides the embedded base during `SongDatabase::load_with_chinese_names`.
- `AnotherSongName.json` — legacy alias map `{ alias: canonical_name }`; imported into search aliases. Bundled as a Tauri resource.
- `HanziKanjiDict.txt` — hanzi→kanji conversion for cross-script search. Bundled as a Tauri resource.

Chinese-name tooling artifacts:
- `song_name_zh.audit.tsv` — committed audit log of the Chinese-name translation workflow (columns: `status source name name_en author candidate name_zh evidence`). Also mirrored under `docs/data/`.
- `song_name_zh.candidates.tsv` — git-ignored working/candidate file produced by the translation workflow.

Files used when rebuilding the local DB:
- `base_song_db.json` — build-time source for the official song DB embedded into the Rust binary with `include_str!`; keyed by `pv_id` string; includes base-game and DLC songs, original name, English name, Chinese name, authors, difficulty, empty aliases, and `source: "base"` or `source: "dlc"`. It is used only by the rebuild flow to merge official songs into `song_db.json`; runtime loading still reads `song_db.json` plus `song_name_zh.json`.
- `pv_db.txt` / `mdata_pv_db.txt` — legacy source files kept for reference and regeneration of `base_song_db.json`; the app no longer reads them during rebuild when using the embedded base DB.

Generated public site:
- `docs/` — static GitHub Pages-compatible Chinese-name database browser containing frontend assets and `docs/data/` copies of public data (`song_db.json`, `song_name_zh.json`, `song_name_zh.audit.tsv`, `manifest.json`).

DLC songs are part of `base_song_db.json` and are marked with `source: "dlc"`. MOD songs are not part of the embedded base DB; they come from mod packs under `mods_dir`.

## Architecture Notes

### Danmaku → Song Request Flow
1. Frontend starts danmaku via `start_danmaku(roomId)` Tauri command.
2. Rust resolves the real room id, fetches WBI keys, signs `getDanmuInfo`, connects to the returned websocket host, and emits `danmaku` / `connection-status` events.
3. If prefix matches (`点歌 <name>`) → trim the text, strip danmaku emoji tokens such as `[喝彩]`, then parse locally and skip LLM.
4. Otherwise, if LLM is enabled → call OpenAI-compatible chat completions and extract only request intent, `song_name`, and `author`; difficulty in danmaku text is intentionally ignored.
5. `SongSearcher` runs multi-pass lookup with optional star-rating filter.
6. First result is added to `SongQueue`; duplicates are rejected by default. Queue entries carry `SearchResult.difficulty_tier`, the actual tier selected by difficulty fallback.
7. Hotkey or UI action dequeues next song and calls `SongSelector::change_song(pv_id, difficulty_tier)`.

### Search Pass Order
1. Japanese/original name (fuzzy)
2. Chinese name (fuzzy, from `song_name_zh.json`)
3. Hanzi→kanji conversion → Japanese name
4. Alias index (`song_db.json` aliases + `AnotherSongName.json`)
5. English name (fuzzy)

### Memory Layout (`song_select.rs`)
Game process: `DivaMegaMix.exe`. Key offsets from `base_address`:
- `0x12B6350` — `LastSelectPVIDMem` (song ID)
- `0x12B634C` — `DifficultySelect` (selected difficulty tab)
- `0x12B6354` — `LastSelectSortMem` (sort mode; set to `1` = difficulty)
- `0x12B635C` — `LastSelectDiffMem` (difficulty; set to `19` = ALL)
- `0xCC61098` — `ChangeSongSelect` (set to `5` to trigger switch)
- `0xCC610A0` — `StartChange` (set to `2`)

**Eden offset quirk**: if `LastSelectPVIDMem` reads `0`, adds `0x105F460` to the select addresses. This handles the "Eden" game variant.

### First-run Wizard
On first launch (no `config.json` next to the executable), the `is_first_run` command returns true and the frontend routes to `src/pages/Wizard.tsx` — a stepped setup that collects room/hotkey/mods/LLM/difficulty preferences, optionally rebuilds the song DB, then saves `config.json` and reloads the window. `room_id` is allowed to be `0` at the end of the wizard (validation is skipped via the `skip_validation` flag on `save_config`).

### Chinese-name Loading
`SongDatabase::load_with_chinese_names` first loads `song_db.json`, then applies the embedded `song_name_zh.json` (compiled in via `include_str!`) as the base, then overrides with the on-disk `data_dir/song_name_zh.json` if present. `SongEntry.name_zh` is `#[serde(skip)]` in `song_db.json`, so Chinese names are never persisted into `song_db.json` by the rebuild flow either — they must be propagated into `song_name_zh.json` (see `merge_chinese_names` in `db_tool.rs`). `BaseSongEntry.name_zh` is `#[serde(default)]`, so `base_song_db.json` itself does carry Chinese names.

## Conventions

- Root directory is the Tauri project root; do not put the active Rust project under `app/`.
- All user-facing strings and comments are in Chinese.
- Keep `config.json` ignored; never commit credentials/cookies.
- Tauri bundled runtime data is listed explicitly in `src-tauri/tauri.conf.json` → `bundle.resources` (`song_db.json`, `AnotherSongName.json`, `HanziKanjiDict.txt`). `base_song_db.json` AND `song_name_zh.json` are compiled into the Rust binary via `include_str!` (in `db_tool.rs` and `song_db.rs`) instead of being shipped as separate runtime resources; the on-disk `song_name_zh.json` only overrides the embedded base.
- VocaDB does **not** reliably provide Chinese song names; do not rely on it for bulk Chinese name population.
