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
6. VocaDB is unreliable for Chinese names in this project. Do not rely on it for bulk population.
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
3. When many names are missing, use `scripts/collect_translation_candidates.py` from this skill to gather review candidates before writing bulk translations. It can query local `song_name_zh.json`, optional public platform suggestions, and manual review URLs, then writes a candidate-review TSV for human review.
4. For each candidate, decide whether it is official/common/uncertain. Platform suggestions are evidence hints, not proof; keep weak candidates as `needs_review`.
5. Write entries under `entries` keyed by original name.
6. Preserve existing audit fields unless intentionally correcting them.
7. Run database/site generation as appropriate:
   - Use the desktop GUI “重建歌曲库” action for local `song_db.json` rebuilds.
   - If public site data changes, keep `docs/data/` copies in sync with `Data/`.
8. Verify JSON parses; run `npm run build` and `cargo check --manifest-path src-tauri/Cargo.toml` if code or bundled data paths changed.

## Bundled Candidate Script

Use the helper when the missing-name set is too large for manual lookup one by one:

```bash
python .opencode/skills/song-chinese-translation/scripts/collect_translation_candidates.py \
  --song-db Data/base_song_db.json \
  --zh-db Data/song_name_zh.json \
  --output Data/song_name_zh.candidates.tsv \
  --search-urls
```

Useful options:

- `--no-web` — avoid network requests and only emit local matches / review URLs.
- `--limit N` — sample the first N missing songs when testing the workflow.
- `--search-urls` — include Bilibili and Moegirl search URLs for manual review.

The candidate TSV is a working file, not `song_name_zh.audit.tsv`. Its columns are optimized for review (`pv_id`, platform, confidence, evidence URL), while the audit TSV keeps the stable project schema:

```text
status	source	name	name_en	author	candidate	name_zh	evidence
```

Review candidate TSV rows before changing JSON. When accepting a candidate, write a normal `song_name_zh.json` entry and then add/update a row in the audit TSV using the stable audit schema above. Do not bulk-mark candidates from public suggestions as `verified` unless you independently confirm they are official or strongly established community names.

## Output Style

When reporting translation work, include:

- number of entries added/changed
- any uncertain entries needing review
- files modified
- verification command/output summary
