use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongRequest {
    pub song_id: u32,
    pub song_name: String,
    pub requester: String,
    pub timestamp: f64,
    pub difficulty: Option<f32>,
    pub difficulty_tier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueSnapshot {
    pub size: usize,
    pub max_size: usize,
    pub allow_duplicates: bool,
    pub songs: Vec<QueuedSong>,
    pub history: Vec<SongRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueuedSong {
    pub position: usize,
    #[serde(flatten)]
    pub request: SongRequest,
}

#[derive(Debug)]
pub struct SongQueue {
    inner: Mutex<SongQueueInner>,
}

#[derive(Debug)]
struct SongQueueInner {
    queue: VecDeque<SongRequest>,
    history: Vec<SongRequest>,
    max_size: usize,
    allow_duplicates: bool,
}

impl SongQueue {
    pub fn new(max_size: usize, allow_duplicates: bool) -> Self {
        Self {
            inner: Mutex::new(SongQueueInner {
                queue: VecDeque::with_capacity(max_size),
                history: Vec::new(),
                max_size,
                allow_duplicates,
            }),
        }
    }

    pub fn add(
        &self,
        song_id: u32,
        song_name: String,
        requester: String,
        difficulty: Option<f32>,
        difficulty_tier: String,
    ) -> bool {
        let mut inner = self.inner();
        if !inner.allow_duplicates && inner.queue.iter().any(|item| item.song_id == song_id) {
            return false;
        }
        if inner.queue.len() >= inner.max_size {
            return false;
        }
        inner.queue.push_back(SongRequest {
            song_id,
            song_name,
            requester,
            timestamp: now_timestamp(),
            difficulty,
            difficulty_tier,
        });
        true
    }

    pub fn next(&self) -> Option<SongRequest> {
        self.inner().queue.pop_front()
    }

    /// 将已经成功切歌的请求写入历史。
    ///
    /// 注意：`next()` 只负责出队，不会立即写入历史。
    /// 这样如果游戏内存写入失败，调用方可以用 `requeue_front()` 把歌曲放回队首，
    /// 避免“切歌失败但歌曲消失并进入历史”的问题。
    pub fn complete(&self, request: SongRequest) {
        self.inner().history.push(request);
    }

    pub fn requeue_front(&self, request: SongRequest) {
        self.inner().queue.push_front(request);
    }

    pub fn list(&self) -> Vec<SongRequest> {
        self.inner().queue.iter().cloned().collect()
    }

    pub fn history(&self) -> Vec<SongRequest> {
        self.inner().history.clone()
    }

    pub fn remove(&self, song_id: u32) -> bool {
        let mut inner = self.inner();
        if let Some(index) = inner.queue.iter().position(|item| item.song_id == song_id) {
            inner.queue.remove(index);
            true
        } else {
            false
        }
    }

    pub fn clear(&self) {
        self.inner().queue.clear();
    }

    pub fn snapshot(&self) -> QueueSnapshot {
        let inner = self.inner();
        QueueSnapshot {
            size: inner.queue.len(),
            max_size: inner.max_size,
            allow_duplicates: inner.allow_duplicates,
            songs: inner
                .queue
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, request)| QueuedSong {
                    position: index + 1,
                    request,
                })
                .collect(),
            history: inner.history.iter().rev().take(5).cloned().collect(),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, SongQueueInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn now_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs_f64())
        .unwrap_or_default()
}
