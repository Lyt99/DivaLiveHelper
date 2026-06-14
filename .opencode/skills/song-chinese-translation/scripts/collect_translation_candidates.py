#!/usr/bin/env python3
"""Collect Chinese title candidates for Project DIVA song-name maintenance.

The script is intentionally dependency-free so it can run from the project root
without setting up a Python environment:

    python .opencode/skills/song-chinese-translation/scripts/collect_translation_candidates.py \
        --song-db Data/base_song_db.json \
        --zh-db Data/song_name_zh.json \
        --output Data/song_name_zh.candidates.tsv

It does not write translations back to the database. Treat the output as review
material: candidates from public platforms are hints, not proof of an official
Chinese title.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen


USER_AGENT = "diva-live-helper-song-name-audit/1.0"


@dataclass(frozen=True)
class Song:
    pv_id: str
    name: str
    name_en: str
    author: str
    source: str


@dataclass(frozen=True)
class Candidate:
    pv_id: str
    name: str
    name_en: str
    author: str
    song_source: str
    candidate: str
    platform: str
    confidence: str
    evidence: str


def load_songs(path: Path) -> list[Song]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    songs = payload.get("songs")
    if not isinstance(songs, dict):
        raise ValueError(f"{path} does not contain a songs object")

    rows: list[Song] = []
    for key, entry in sorted(songs.items(), key=lambda item: int(item[0])):
        if (entry.get("name_zh") or "").strip():
            continue
        authors = entry.get("authors") or []
        rows.append(
            Song(
                pv_id=str(key),
                name=str(entry.get("name") or ""),
                name_en=str(entry.get("name_en") or ""),
                author=",".join(str(author) for author in authors),
                source=str(entry.get("source") or ""),
            )
        )
    return rows


def load_existing_zh(path: Path) -> dict[str, dict[str, Any]]:
    if not path.exists():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    entries = payload.get("entries") or {}
    if not isinstance(entries, dict):
        raise ValueError(f"{path} entries must be an object")
    return entries


def http_json(url: str, timeout: float) -> Any:
    request = Request(url, headers={"User-Agent": USER_AGENT})
    with urlopen(request, timeout=timeout) as response:  # noqa: S310 - user-requested public lookups only
        return json.loads(response.read().decode("utf-8", errors="replace"))


def local_candidates(song: Song, zh_entries: dict[str, dict[str, Any]]) -> list[Candidate]:
    candidates: list[Candidate] = []
    exact = zh_entries.get(song.name)
    if exact and exact.get("name_zh"):
        candidates.append(
            Candidate(
                song.pv_id,
                song.name,
                song.name_en,
                song.author,
                song.source,
                str(exact["name_zh"]),
                "local-song_name_zh",
                "high",
                f"exact key match; status={exact.get('status', '')}; evidence={exact.get('evidence', '')}",
            )
        )

    lowered_en = song.name_en.strip().casefold()
    if lowered_en:
        for key, entry in zh_entries.items():
            if str(entry.get("name_en") or "").strip().casefold() == lowered_en and entry.get("name_zh"):
                candidates.append(
                    Candidate(
                        song.pv_id,
                        song.name,
                        song.name_en,
                        song.author,
                        song.source,
                        str(entry["name_zh"]),
                        "local-song_name_zh",
                        "medium",
                        f"same English title as {key}; status={entry.get('status', '')}",
                    )
                )
                break
    return candidates


def moegirl_candidates(song: Song, timeout: float) -> list[Candidate]:
    query = song.name or song.name_en
    if not query:
        return []
    url = "https://zh.moegirl.org.cn/api.php?" + urlencode(
        {
            "action": "opensearch",
            "namespace": "0",
            "limit": "5",
            "format": "json",
            "search": query,
        }
    )
    payload = http_json(url, timeout)
    titles = payload[1] if isinstance(payload, list) and len(payload) > 1 else []
    links = payload[3] if isinstance(payload, list) and len(payload) > 3 else []
    candidates: list[Candidate] = []
    for index, title in enumerate(titles[:5]):
        evidence = links[index] if index < len(links) else url
        candidates.append(
            Candidate(
                song.pv_id,
                song.name,
                song.name_en,
                song.author,
                song.source,
                str(title),
                "moegirl-opensearch",
                "medium" if title == song.name else "low",
                str(evidence),
            )
        )
    return candidates


def bilibili_candidates(song: Song, timeout: float) -> list[Candidate]:
    query = song.name or song.name_en
    if not query:
        return []
    url = "https://s.search.bilibili.com/main/suggest?" + urlencode({"term": query, "main_ver": "v1"})
    payload = http_json(url, timeout)
    result = payload.get("result") if isinstance(payload, dict) else None
    candidates: list[Candidate] = []
    if isinstance(result, dict):
        tags = result.get("tag") or []
        if isinstance(tags, list):
            for item in tags[:5]:
                if not isinstance(item, dict):
                    continue
                value = item.get("value") or item.get("name")
                if not value:
                    continue
                candidates.append(
                    Candidate(
                        song.pv_id,
                        song.name,
                        song.name_en,
                        song.author,
                        song.source,
                        str(value),
                        "bilibili-suggest",
                        "low",
                        f"suggestion API for query={query}",
                    )
                )
    return candidates


def search_url_candidates(song: Song) -> list[Candidate]:
    query = song.name or song.name_en
    if not query:
        return []
    encoded = quote(query)
    return [
        Candidate(
            song.pv_id,
            song.name,
            song.name_en,
            song.author,
            song.source,
            "",
            "review-url",
            "manual",
            f"https://search.bilibili.com/all?keyword={encoded}",
        ),
        Candidate(
            song.pv_id,
            song.name,
            song.name_en,
            song.author,
            song.source,
            "",
            "review-url",
            "manual",
            f"https://zh.moegirl.org.cn/index.php?search={encoded}",
        ),
    ]


def collect(song: Song, zh_entries: dict[str, dict[str, Any]], args: argparse.Namespace) -> list[Candidate]:
    candidates = local_candidates(song, zh_entries)
    if args.search_urls:
        candidates.extend(search_url_candidates(song))
    if args.no_web:
        return candidates

    for fetcher in (moegirl_candidates, bilibili_candidates):
        try:
            candidates.extend(fetcher(song, args.timeout))
            time.sleep(args.delay)
        except (TimeoutError, URLError, OSError, json.JSONDecodeError) as error:
            candidates.append(
                Candidate(
                    song.pv_id,
                    song.name,
                    song.name_en,
                    song.author,
                    song.source,
                    "",
                    fetcher.__name__.replace("_candidates", ""),
                    "error",
                    str(error),
                )
            )
    return candidates


def write_tsv(path: Path, rows: list[Candidate]) -> None:
    header = [
        "pv_id",
        "name",
        "name_en",
        "author",
        "song_source",
        "platform",
        "confidence",
        "candidate",
        "evidence",
    ]

    def clean(value: str) -> str:
        return value.replace("\t", " ").replace("\r", " ").replace("\n", " ")

    lines = ["\t".join(header)]
    for row in rows:
        lines.append(
            "\t".join(
                clean(value)
                for value in [
                    row.pv_id,
                    row.name,
                    row.name_en,
                    row.author,
                    row.song_source,
                    row.platform,
                    row.confidence,
                    row.candidate,
                    row.evidence,
                ]
            )
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--song-db", type=Path, default=Path("Data/base_song_db.json"))
    parser.add_argument("--zh-db", type=Path, default=Path("Data/song_name_zh.json"))
    parser.add_argument("--output", type=Path, default=Path("Data/song_name_zh.candidates.tsv"))
    parser.add_argument("--limit", type=int, default=0, help="Only process the first N missing songs")
    parser.add_argument("--no-web", action="store_true", help="Only use local matches and review URLs")
    parser.add_argument("--search-urls", action="store_true", help="Include manual review search URLs")
    parser.add_argument("--timeout", type=float, default=8.0)
    parser.add_argument("--delay", type=float, default=0.2, help="Delay between public lookup requests")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    songs = load_songs(args.song_db)
    if args.limit > 0:
        songs = songs[: args.limit]
    zh_entries = load_existing_zh(args.zh_db)
    rows: list[Candidate] = []
    for song in songs:
        rows.extend(collect(song, zh_entries, args))
    write_tsv(args.output, rows)
    print(
        json.dumps(
            {
                "songs_checked": len(songs),
                "candidates_written": len(rows),
                "output": str(args.output),
            },
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
