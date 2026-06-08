"""
diva-translate — 歌曲中文名翻译工具

独立运行，不依赖直播间连接。
使用 OpenAI 兼容的 LLM API 翻译日文/英文歌名，并写入 song_name_zh.json。

用法（单首查询）：
    uv run diva-translate "初音ミクの消失"
    uv run diva-translate "Tell Your World" "Sharing The World"

用法（批量查询，从文件读取，每行一首）：
    uv run diva-translate --file songs.txt

其他选项：
    uv run diva-translate "Miku" --no-cache          # 忽略本地缓存，强制重新翻译
    uv run diva-translate "Miku" --proxy http://127.0.0.1:7890
    uv run diva-translate "Miku" --config path/to/config.json
    uv run diva-translate "Miku" --api-key sk-xxx --base-url https://api.deepseek.com --model deepseek-chat
    uv run diva-translate --list-cache               # 显示当前 song_name_zh.json 内容
    uv run diva-translate --all                      # 翻译 song_db.json 中所有尚无中文名的歌曲
"""

import argparse
import asyncio
import sys
from pathlib import Path


def _configure_console_encoding():
    """尽量让 Windows 控制台按 UTF-8 输出中文和日文。"""
    try:
        stdout_reconfigure = getattr(sys.stdout, "reconfigure", None)
        stderr_reconfigure = getattr(sys.stderr, "reconfigure", None)
        if callable(stdout_reconfigure):
            stdout_reconfigure(encoding="utf-8")
        if callable(stderr_reconfigure):
            stderr_reconfigure(encoding="utf-8")
    except Exception:
        pass


def _load_names_from_file(path: str) -> list[str]:
    """从文件中读取歌曲名（每行一个，忽略空行和 # 注释行）"""
    file = Path(path)
    if not file.exists():
        print(f"错误: 文件不存在: {path}", file=sys.stderr)
        sys.exit(1)
    lines = file.read_text(encoding="utf-8").splitlines()
    return [ln.strip() for ln in lines if ln.strip() and not ln.strip().startswith("#")]


def _print_zh_db(zh_db_path: Path):
    """打印 song_name_zh.json 内容"""
    from diva_live_helper.song_db import ChineseNameDatabase

    zh_db = ChineseNameDatabase(zh_db_path)
    if not zh_db.load():
        print("song_name_zh.json 不存在或为空。")
        return
    if not zh_db.names:
        print("中文名数据库为空。")
        return
    print(f"共 {len(zh_db.names)} 条中文名：")
    for name, zh in sorted(zh_db.names.items()):
        entry = zh_db.entries.get(name)
        evidence = f" [{entry.evidence}]" if entry and entry.evidence else ""
        print(f"  {name}  ->  {zh}{evidence}")


async def _run_query(
    names: list[str],
    data_dir: str,
    api_key: str,
    base_url: str,
    model: str,
    proxy: str,
    no_cache: bool,
    write_to_zh_db: bool,
) -> dict[str, str | None]:
    """
    异步翻译歌曲名，返回 {原名: 中文名 or None}。

    如果 write_to_zh_db=True，会将结果按原曲名写入 song_name_zh.json。
    """
    from diva_live_helper.translator import SongTranslator

    translator = SongTranslator(
        api_key=api_key,
        base_url=base_url,
        model=model,
        proxy=proxy,
        cache_dir=data_dir,
    )

    if no_cache:
        translator.cache = {}

    results: dict[str, str | None] = {}

    if len(names) == 1:
        name = names[0]
        zh = await translator.translate_name(name)
        results[name] = zh
        if zh:
            translator.save_cache()
    else:
        batch_results = await translator.batch_translate(names)
        for name in names:
            results[name] = batch_results.get(name)

    await translator.close()

    # 将翻译结果写入 song_name_zh.json（按原曲名存储）
    if write_to_zh_db:
        _write_to_zh_db(results, data_dir)

    return results


def _write_to_zh_db(results: dict[str, str | None], data_dir: str):
    """将翻译结果写入 song_name_zh.json（key 为原曲名）"""
    from diva_live_helper.song_db import ChineseNameDatabase, SongDatabase

    data_path = Path(data_dir)
    db_path = data_path / "song_db.json"
    zh_db_path = data_path / "song_name_zh.json"

    if not db_path.exists():
        print("警告: song_db.json 不存在，无法写入 song_name_zh.json，仅更新缓存文件。")
        return

    song_db = SongDatabase(db_path)
    if not song_db.load():
        print("警告: 加载 song_db.json 失败，仅更新缓存文件。")
        return

    zh_db = ChineseNameDatabase(zh_db_path)
    zh_db.load()

    # 构建 name/name_en -> 原曲名(name) 反查索引；当前中文名库按原曲名存储。
    name_to_original_name: dict[str, str] = {}
    for entry in song_db.songs.values():
        if entry.name:
            name_to_original_name[entry.name] = entry.name
        if entry.name_en and entry.name:
            name_to_original_name[entry.name_en] = entry.name

    updated = 0
    for name, zh in results.items():
        if not zh:
            continue
        original_name = name_to_original_name.get(name)
        if original_name:
            matched_entry = next(
                (entry for entry in song_db.songs.values() if entry.name == original_name),
                None,
            )
            zh_db.set_name(
                original_name,
                zh,
                source=matched_entry.source if matched_entry else "",
                name_en=matched_entry.name_en if matched_entry else "",
                author=",".join(matched_entry.authors) if matched_entry else "",
                candidate="diva-translate",
                evidence="llm-generated",
            )
            updated += 1

    if updated:
        zh_db.save()
        print(f"已将 {updated} 条中文名写入 {zh_db_path}")
    else:
        print("未能将翻译结果写入 song_name_zh.json（歌名未在 song_db.json 中匹配到）。")


async def _run_translate_all(
    data_dir: str,
    api_key: str,
    base_url: str,
    model: str,
    proxy: str,
    no_cache: bool,
    batch_size: int,
) -> int:
    """翻译 song_db.json 中所有尚无中文名的歌曲，写入 song_name_zh.json"""
    from diva_live_helper.song_db import ChineseNameDatabase, SongDatabase
    from diva_live_helper.translator import SongTranslator

    data_path = Path(data_dir)
    db_path = data_path / "song_db.json"
    zh_db_path = data_path / "song_name_zh.json"

    if not db_path.exists():
        print(f"错误: song_db.json 不存在: {db_path}", file=sys.stderr)
        return 0

    song_db = SongDatabase(db_path)
    if not song_db.load():
        print("错误: 加载 song_db.json 失败。", file=sys.stderr)
        return 0

    zh_db = ChineseNameDatabase(zh_db_path)
    zh_db.load()

    # 收集尚无中文名的歌曲
    missing: list[str] = []  # 原曲名列表
    for entry in song_db.songs.values():
        if entry.name in zh_db.names and not no_cache:
            continue
        # 优先用日文名，次选英文名
        display_name = entry.name or entry.name_en
        if display_name:
            missing.append(display_name)

    if not missing:
        print("所有歌曲都已有中文名，无需翻译。")
        return 0

    print(f"发现 {len(missing)} 首歌曲需要翻译...")

    translator = SongTranslator(
        api_key=api_key,
        base_url=base_url,
        model=model,
        proxy=proxy,
        cache_dir=data_dir,
    )
    if no_cache:
        translator.cache = {}

    batch_results = await translator.batch_translate(missing, batch_size=batch_size)
    await translator.close()

    updated = 0
    for name, zh in batch_results.items():
        if zh:
            matched_entry = next((entry for entry in song_db.songs.values() if entry.name == name), None)
            zh_db.set_name(
                name,
                zh,
                source=matched_entry.source if matched_entry else "",
                name_en=matched_entry.name_en if matched_entry else "",
                author=",".join(matched_entry.authors) if matched_entry else "",
                candidate="diva-translate",
                evidence="llm-generated",
            )
            updated += 1

    if updated:
        zh_db.save()
        print(f"已更新 {updated} 条中文名 → {zh_db_path}")

    return updated


def main():
    _configure_console_encoding()

    parser = argparse.ArgumentParser(
        prog="diva-translate",
        description="翻译歌曲中文名（LLM），结果写入 song_name_zh.json",
    )
    parser.add_argument(
        "names",
        nargs="*",
        metavar="歌曲名",
        help="要翻译的歌曲名（可传多个）",
    )
    parser.add_argument(
        "--file",
        metavar="PATH",
        default=None,
        help="从文件批量读取歌曲名（每行一个）",
    )
    parser.add_argument(
        "--all",
        action="store_true",
        default=False,
        help="翻译 song_db.json 中所有尚无中文名的歌曲",
    )
    parser.add_argument(
        "--no-cache",
        action="store_true",
        default=False,
        help="忽略本地缓存，强制重新翻译",
    )
    parser.add_argument(
        "--list-cache",
        action="store_true",
        default=False,
        help="显示 song_name_zh.json 当前内容后退出",
    )
    parser.add_argument(
        "--batch-size",
        metavar="N",
        type=int,
        default=20,
        help="每批翻译的歌曲数量（默认 20，仅 --all 或批量模式有效）",
    )
    parser.add_argument(
        "--config",
        metavar="PATH",
        default="config.json",
        help="配置文件路径（默认 config.json）",
    )
    parser.add_argument(
        "--data-dir",
        metavar="DIR",
        default=None,
        help="数据文件目录（覆盖 config.json 中的 data_dir）",
    )
    parser.add_argument(
        "--proxy",
        metavar="URL",
        default=None,
        help="HTTP 代理地址（覆盖 config.json 中的 http_proxy）",
    )
    parser.add_argument(
        "--api-key",
        metavar="KEY",
        default=None,
        help="LLM API Key（覆盖 config.json 中的 llm_api_key）",
    )
    parser.add_argument(
        "--base-url",
        metavar="URL",
        default=None,
        help="LLM API Base URL（覆盖 config.json 中的 llm_base_url）",
    )
    parser.add_argument(
        "--model",
        metavar="NAME",
        default=None,
        help="LLM 模型名称（覆盖 config.json 中的 llm_model）",
    )

    args = parser.parse_args()

    # 加载配置
    from diva_live_helper.config import Config

    config = Config()
    config.load(args.config)

    data_dir = args.data_dir or config.data_dir
    proxy = args.proxy if args.proxy is not None else config.http_proxy
    api_key = args.api_key if args.api_key is not None else config.llm_api_key
    base_url = args.base_url if args.base_url is not None else config.llm_base_url
    model = args.model if args.model is not None else config.llm_model

    zh_db_path = Path(data_dir) / "song_name_zh.json"

    # --list-cache 模式
    if args.list_cache:
        _print_zh_db(zh_db_path)
        return

    # 检查 API Key
    if not api_key:
        print(
            "错误: 未提供 LLM API Key。\n"
            "请在 config.json 中设置 llm_api_key，或使用 --api-key 参数。",
            file=sys.stderr,
        )
        sys.exit(1)

    print("=== diva-translate ===")
    print(f"数据目录  : {data_dir}")
    print(f"LLM       : {base_url} / {model}")
    if proxy:
        print(f"代理      : {proxy}")
    if args.no_cache:
        print("缓存      : 已禁用（强制重新翻译）")
    print()

    # --all 模式：翻译所有缺少中文名的歌曲
    if args.all:
        updated = asyncio.run(
            _run_translate_all(
                data_dir=data_dir,
                api_key=api_key,
                base_url=base_url,
                model=model,
                proxy=proxy,
                no_cache=args.no_cache,
                batch_size=args.batch_size,
            )
        )
        print(f"\n共更新 {updated} 首歌曲的中文名。")
        return

    # 收集要翻译的歌曲名
    names: list[str] = list(args.names)
    if args.file:
        names.extend(_load_names_from_file(args.file))

    if not names:
        parser.print_help()
        print("\n错误: 请提供至少一个歌曲名，或使用 --file 指定文件，或使用 --all 翻译全部。", file=sys.stderr)
        sys.exit(1)

    # 去重，保留顺序
    seen: set[str] = set()
    unique_names: list[str] = []
    for n in names:
        if n not in seen:
            seen.add(n)
            unique_names.append(n)

    # 执行翻译（多首时写入 song_name_zh.json）
    results = asyncio.run(
        _run_query(
            names=unique_names,
            data_dir=data_dir,
            api_key=api_key,
            base_url=base_url,
            model=model,
            proxy=proxy,
            no_cache=args.no_cache,
            write_to_zh_db=len(unique_names) >= 1,
        )
    )

    # 输出结果
    found = 0
    not_found = 0
    for name in unique_names:
        zh = results.get(name)
        if zh:
            print(f"  [OK]  {name}  ->  {zh}")
            found += 1
        else:
            print(f"  [MISS]  {name}  ->  （未找到中文名）")
            not_found += 1

    print()
    print(f"查询完成：{found} 首找到中文名，{not_found} 首未找到。")
    if found:
        print(f"结果已写入: {zh_db_path}")


if __name__ == "__main__":
    main()
