# Song Chinese Translation

Use this skill when adding, reviewing, or maintaining Chinese song names for this project, especially `Data/song_name_zh.json`, `song_name_zh.audit.tsv`, or generated docs data.

## Purpose

Maintain a shareable, auditable Chinese-name database for Project DIVA songs. The goal is search usefulness for Chinese-speaking users, not literal machine translation.

## Source of Truth

- Primary song DB: `Data/song_db.json`
- Chinese names: `Data/song_name_zh.json`
- Public site is generated output under `docs/`; do not treat it as source of truth.
- `song_name_zh.json` uses version 3 format:

```json
{
  "version": 3,
  "entries": {
    "原曲名": {
      "name_zh": "中文名",
      "status": "verified|auto|needs_review",
      "source": "来源说明",
      "name_en": "英文名（如有）",
      "author": "作者（如有）",
      "candidate": "候选/检索线索",
      "evidence": "证据说明"
    }
  }
}
```

Keys are the original `song_db.json` `name` field, **not pv_id**, because pv_id can collide across mods.

## Translation Rules

1. Prefer widely used Chinese community names over literal translations.
2. Preserve official stylization when it is recognizable and useful for search.
3. Do not invent a Chinese title when evidence is weak. Use `status: "needs_review"` if uncertain.
4. Keep original-language names as keys exactly as they appear in `song_db.json`.
5. Do not write Chinese names into `song_db.json`; it is a generated structural DB and should remain shareable.
6. VocaDB is unreliable for Chinese names in this project. Do not rely on `translator.py` / `diva-translate` for bulk population.
7. Include aliases/search variants in `AnotherSongName.json` or song aliases, not as replacement Chinese names.

## Evidence Standard

For each new or changed entry, record enough evidence for future review:

- `status: "verified"` for official or strongly established community names.
- `status: "auto"` for migrated/cache-derived names that are plausible but not manually reviewed.
- `status: "needs_review"` for uncertain translations.
- `source` should name the evidence source category, e.g. `official`, `community`, `manual`, `song-name-cache`.
- `evidence` should briefly explain where the name came from or why it was chosen.

## Workflow

1. Load `Data/song_db.json` and identify original song names needing Chinese names.
2. Check existing `Data/song_name_zh.json` before adding anything.
3. For each candidate, decide whether it is official/common/uncertain.
4. Write entries under `entries` keyed by original name.
5. Preserve existing audit fields unless intentionally correcting them.
6. Run database/site generation as appropriate:
   - Python workflow: `uv run update_db`, then optionally `uv run build-zh-site`
   - Tauri workflow: use GUI “重建歌曲库” for local `song_db.json` rebuild; use Python `build-zh-site` for public site output.
7. Verify JSON parses and spot-check search behavior if code changed.

## Output Style

When reporting translation work, include:

- number of entries added/changed
- any uncertain entries needing review
- files modified
- verification command/output summary
