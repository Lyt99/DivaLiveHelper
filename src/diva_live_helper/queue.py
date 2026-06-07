"""
点歌队列管理模块
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass
from threading import RLock


SongValue = int | float | str | None
SongDict = dict[str, SongValue]
QueueSnapshot = dict[str, int | float | bool | list[SongDict]]


@dataclass
class SongRequest:
    """点歌请求"""
    song_id: int
    song_name: str
    requester: str
    timestamp: float = 0.0
    # 点歌时指定的难度星级（None = 未指定）
    difficulty: float | None = None

    def __post_init__(self):
        if self.timestamp == 0.0:
            import time
            self.timestamp = time.time()

    def to_dict(self) -> SongDict:
        """转换为可序列化字典，用于 OBS 队列展示。"""
        return {
            "song_id": self.song_id,
            "song_name": self.song_name,
            "requester": self.requester,
            "timestamp": self.timestamp,
            "difficulty": self.difficulty,
        }


class SongQueue:
    """点歌队列"""
    
    def __init__(self, max_size: int = 50, allow_duplicates: bool = False):
        self.queue: deque[SongRequest] = deque(maxlen=max_size)
        self.max_size: int = max_size
        self.allow_duplicates: bool = allow_duplicates
        self.history: list[SongRequest] = []  # 播放历史
        self._lock: RLock = RLock()
    
    def add(
        self,
        song_id: int,
        song_name: str,
        requester: str,
        difficulty: float | None = None,
    ) -> bool:
        """
        添加歌曲到队列

        Args:
            song_id: 歌曲ID
            song_name: 歌曲名
            requester: 点歌人
            difficulty: 点歌时指定的难度星级（None = 未指定）

        Returns:
            是否添加成功
        """
        with self._lock:
            if not self.allow_duplicates:
                for item in self.queue:
                    if item.song_id == song_id:
                        return False

            if len(self.queue) >= self.max_size:
                return False

            request = SongRequest(
                song_id=song_id,
                song_name=song_name,
                requester=requester,
                difficulty=difficulty,
            )
            self.queue.append(request)
            return True
    
    def next(self) -> tuple[int, str, str, float | None] | None:
        """
        从队列取出下一首歌

        Returns:
            (歌曲ID, 歌曲名, 点歌人, 难度星级或None) 或 None
        """
        with self._lock:
            if not self.queue:
                return None
            request = self.queue.popleft()
            self.history.append(request)
            return request.song_id, request.song_name, request.requester, request.difficulty

    def peek(self) -> tuple[int, str, str, float | None] | None:
        """
        查看队列中的下一首歌（不取出）

        Returns:
            (歌曲ID, 歌曲名, 点歌人, 难度星级或None) 或 None
        """
        with self._lock:
            if not self.queue:
                return None
            request = self.queue[0]
            return request.song_id, request.song_name, request.requester, request.difficulty
    
    def remove(self, song_id: int) -> bool:
        """
        从队列中移除指定歌曲
        
        Args:
            song_id: 歌曲ID
        
        Returns:
            是否移除成功
        """
        with self._lock:
            for i, item in enumerate(self.queue):
                if item.song_id == song_id:
                    del self.queue[i]
                    return True
            return False
    
    def clear(self):
        """清空队列"""
        with self._lock:
            self.queue.clear()
    
    def is_empty(self) -> bool:
        """检查队列是否为空"""
        with self._lock:
            return len(self.queue) == 0
    
    def size(self) -> int:
        """获取队列大小"""
        with self._lock:
            return len(self.queue)
    
    def get_queue_list(self) -> list[tuple[int, str, str, float | None]]:
        """获取队列列表，每个元素为 (歌曲ID, 歌曲名, 点歌人, 难度或None)"""
        with self._lock:
            return [
                (item.song_id, item.song_name, item.requester, item.difficulty)
                for item in self.queue
            ]

    def get_history(self) -> list[tuple[int, str, str, float | None]]:
        """获取播放历史，每个元素为 (歌曲ID, 歌曲名, 点歌人, 难度或None)"""
        with self._lock:
            return [
                (item.song_id, item.song_name, item.requester, item.difficulty)
                for item in self.history
            ]

    def snapshot(self) -> QueueSnapshot:
        """获取 OBS 展示用的队列快照。"""
        with self._lock:
            return {
                "size": len(self.queue),
                "max_size": self.max_size,
                "allow_duplicates": self.allow_duplicates,
                "songs": [
                    {"position": index, **item.to_dict()}
                    for index, item in enumerate(self.queue, start=1)
                ],
                "history": [item.to_dict() for item in self.history[-5:]],
            }
    
    def clear_history(self):
        """清空播放历史"""
        with self._lock:
            self.history.clear()
