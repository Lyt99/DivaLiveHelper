"""
歌曲名翻译模块
使用 OpenAI 兼容的 LLM API 将日文/英文歌名翻译为中文
"""

import asyncio
import json
from pathlib import Path
from typing import Optional

from openai import AsyncOpenAI

# 单首翻译系统提示
_SYSTEM_PROMPT_SINGLE = """你是一个专业的日文音乐作品翻译助手，专注于 VOCALOID（初音未来等虚拟歌手）歌曲。
给定一个日文或英文歌曲名，返回其最常用的中文译名。

规则：
1. 返回简短的中文译名（通常是官方译名或社区广泛使用的版本）
2. 如果歌名本身是汉字或中文，直接原样返回
3. 如果找不到合适的中文译名，返回 null
4. 只返回 JSON，格式：{"name_zh": "中文名或null"}
5. 不要添加任何解释或额外文字"""

# 批量翻译系统提示
_SYSTEM_PROMPT_BATCH = """你是一个专业的日文音乐作品翻译助手，专注于 VOCALOID（初音未来等虚拟歌手）歌曲。
给定一组日文或英文歌曲名（JSON 数组），为每首歌返回最常用的中文译名。

规则：
1. 返回简短的中文译名（通常是官方译名或社区广泛使用的版本）
2. 如果歌名本身是汉字或中文，直接原样返回
3. 如果某首歌找不到合适的中文译名，对应值填 null
4. 只返回 JSON 对象，键为原歌名，值为中文名或 null
5. 不要添加任何解释或额外文字
示例输出：{"Love is War": "恋爱战争", "初音ミクの消失": "初音未来的消失", "unknown song": null}"""


class SongTranslator:
    """歌曲名翻译器 - 使用 OpenAI 兼容的 LLM API"""

    def __init__(
        self,
        api_key: str,
        base_url: str = "https://api.deepseek.com",
        model: str = "deepseek-chat",
        proxy: str = "",
        cache_dir: str = "Data",
    ):
        """
        Args:
            api_key: LLM API Key
            base_url: OpenAI 兼容 API 地址
            model: 模型名称
            proxy: HTTP 代理地址（可选）
            cache_dir: 缓存目录（用于读写 song_name_cache.json 临时缓存）
        """
        try:
            import httpx

            if proxy:
                http_client = httpx.AsyncClient(proxy=proxy, trust_env=False)
            else:
                http_client = httpx.AsyncClient(trust_env=False)
        except Exception:
            http_client = None

        self.client = AsyncOpenAI(
            api_key=api_key,
            base_url=base_url,
            http_client=http_client,
        )
        self.model = model
        self.cache_dir = Path(cache_dir)
        self.cache_file = self.cache_dir / "song_name_cache.json"

        # 内存缓存: {原文名: 中文名}
        self.cache: dict[str, str] = {}
        self._load_cache()

    def _load_cache(self):
        """加载本地缓存"""
        if self.cache_file.exists():
            try:
                with open(self.cache_file, "r", encoding="utf-8") as f:
                    self.cache = json.load(f)
                print(f"已加载 {len(self.cache)} 条歌曲名缓存")
            except Exception as e:
                print(f"加载缓存失败: {e}")
                self.cache = {}

    def _save_cache(self):
        """保存缓存到本地"""
        try:
            self.cache_dir.mkdir(parents=True, exist_ok=True)
            with open(self.cache_file, "w", encoding="utf-8") as f:
                json.dump(self.cache, f, ensure_ascii=False, indent=2)
        except Exception as e:
            print(f"保存缓存失败: {e}")

    def get_cached_name(self, song_name: str) -> Optional[str]:
        """从缓存获取中文名"""
        return self.cache.get(song_name)

    async def translate_name(self, song_name: str) -> Optional[str]:
        """
        将单首歌曲名翻译为中文。

        Args:
            song_name: 日文名或英文名

        Returns:
            中文名，如果翻译失败或无对应译名返回 None
        """
        # 先查缓存
        cached = self.get_cached_name(song_name)
        if cached:
            return cached

        # 调用 LLM
        result = await self._llm_translate_single(song_name)
        if result:
            self.cache[song_name] = result
        return result

    async def _llm_translate_single(self, song_name: str) -> Optional[str]:
        """调用 LLM 翻译单首歌名"""
        try:
            response = await self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": _SYSTEM_PROMPT_SINGLE},
                    {"role": "user", "content": song_name},
                ],
                response_format={"type": "json_object"},
                temperature=0.1,
                max_tokens=2000,
            )
            print(response)
            content = response.choices[0].message.content
            if not content:
                return None
            data = json.loads(content)
            name_zh = data.get("name_zh")
            if isinstance(name_zh, str) and name_zh.strip():
                return name_zh.strip()
            return None
        except Exception as e:
            print(f"LLM 翻译失败 [{song_name}]: {e}")
            return None

    async def batch_translate(
        self,
        song_names: list[str],
        batch_size: int = 20,
    ) -> dict[str, str]:
        """
        批量翻译歌曲名（将多首歌合并成一次 LLM 调用，减少 API 请求次数）。

        Args:
            song_names: 歌曲名列表
            batch_size: 每批翻译数量（默认 20 首合并一次调用）

        Returns:
            {原名: 中文名} 映射（只包含成功翻译的条目）
        """
        results: dict[str, str] = {}
        uncached: list[str] = []

        # 先命中缓存
        for name in song_names:
            cached = self.get_cached_name(name)
            if cached:
                results[name] = cached
            else:
                uncached.append(name)

        if not uncached:
            return results

        print(f"需要翻译 {len(uncached)} 首歌曲...")

        # 分批调用 LLM
        for i in range(0, len(uncached), batch_size):
            batch = uncached[i : i + batch_size]
            batch_results = await self._llm_translate_batch(batch)
            for name, zh in batch_results.items():
                results[name] = zh
                self.cache[name] = zh
            done = min(i + batch_size, len(uncached))
            print(f"  已翻译 {done}/{len(uncached)}")

        # 保存缓存
        self._save_cache()
        print(f"翻译完成，共 {len(results)} 首歌曲有中文名")
        return results

    async def _llm_translate_batch(self, song_names: list[str]) -> dict[str, str]:
        """调用 LLM 批量翻译歌名，返回 {原名: 中文名}（仅含成功的）"""
        if not song_names:
            return {}
        try:
            user_content = json.dumps(song_names, ensure_ascii=False)
            response = await self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": _SYSTEM_PROMPT_BATCH},
                    {"role": "user", "content": user_content},
                ],
                response_format={"type": "json_object"},
                temperature=0.1,
                max_tokens=len(song_names) * 100 + 2000,
            )
            content = response.choices[0].message.content
            if not content:
                return {}
            data = json.loads(content)
            return {
                name: zh
                for name, zh in data.items()
                if isinstance(zh, str) and zh.strip()
            }
        except Exception as e:
            print(f"LLM 批量翻译失败（{len(song_names)} 首）: {e}")
            # 降级：逐首翻译
            results: dict[str, str] = {}
            for name in song_names:
                zh = await self._llm_translate_single(name)
                if zh:
                    results[name] = zh
                await asyncio.sleep(0.2)
            return results

    def save_cache(self):
        """手动保存缓存"""
        self._save_cache()
        print(f"已保存 {len(self.cache)} 条缓存")

    async def close(self):
        """关闭 LLM 客户端"""
        await self.client.close()
