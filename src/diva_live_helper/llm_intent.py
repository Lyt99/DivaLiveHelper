"""
LLM意图识别模块
通过大模型分析弹幕语义，判断是否是点歌意图，并提取歌名、作者名、难度要求
"""

import json
from pydantic import BaseModel

try:
    from openai import AsyncOpenAI
except ImportError:
    AsyncOpenAI = None


class SongIntent(BaseModel):
    """点歌意图识别结果"""
    is_song_request: bool
    song_name: str | None = None
    author: str | None = None      # 作者名（点该作者的歌）
    difficulty: float | None = None  # 难度星级，如 7.0、8.5


SYSTEM_PROMPT = """你是一个B站直播间的点歌助手。分析弹幕是否包含点歌意图，提取歌名、作者名、难度要求。

分析规则：
1. 明确想听某首歌（"点歌xxx"、"我想听xxx"、"来一首xxx"、"放xxx"等）→ is_song_request=true
2. 普通聊天、刷屏、表情等 → is_song_request=false
3. song_name：歌曲名称本身，去掉"点歌"等前缀。若只提到作者没提歌名则为null
4. author：若弹幕提到"xxx的歌"、"来首xxx的"等，填入作者名；若没提作者则为null
5. difficulty：若提到难度星级（"7星"、"8.5星"、"难度8"等），填入对应浮点数；
   若提到难度档位（"ex"/"extreme"→按extreme档，"hard"→按hard档，"normal"→按normal档，"easy"→按easy档），
   也填入该难度档对应的星级范围中值（easy≈3, normal≈5, hard≈7, extreme≈9）；
   没有难度要求则为null
6. 歌名中含有难度关键词时（如"Love is War ex"），song_name去掉难度部分只保留歌名，难度放入difficulty

输出必须是JSON，不要其他文字：
{"is_song_request": true/false, "song_name": "歌名或null", "author": "作者名或null", "difficulty": 数字或null}"""


class LLMIntentAnalyzer:
    """LLM意图分析器"""

    def __init__(
        self,
        api_key: str,
        base_url: str = "https://api.deepseek.com",
        model: str = "deepseek-chat",
        proxy: str = "",
    ):
        if AsyncOpenAI is None:
            raise ImportError("openai库未安装，请运行: uv add openai")

        http_client = None
        if proxy:
            try:
                import httpx
                http_client = httpx.AsyncClient(proxy=proxy)
            except Exception:
                pass

        self.client = AsyncOpenAI(
            api_key=api_key,
            base_url=base_url,
            http_client=http_client,
        )
        self.model = model

    async def analyze(self, message: str) -> SongIntent:
        """
        分析弹幕是否是点歌意图

        Args:
            message: 弹幕内容

        Returns:
            SongIntent 结果，包含 song_name、author、difficulty
        """
        try:
            response = await self.client.chat.completions.create(
                model=self.model,
                messages=[
                    {"role": "system", "content": SYSTEM_PROMPT},
                    {"role": "user", "content": message},
                ],
                response_format={"type": "json_object"},
                temperature=0.1,
                max_tokens=150,
            )

            content = response.choices[0].message.content
            if content is None:
                return SongIntent(is_song_request=False)
            data = json.loads(content)
            return SongIntent(**data)

        except Exception as e:
            print(f"LLM意图分析失败: {e}")
            return SongIntent(is_song_request=False)

    async def close(self):
        """关闭客户端"""
        await self.client.close()
