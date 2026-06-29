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

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一条测试歌曲请求，简化测试样板
    fn make_request(song_id: u32) -> SongRequest {
        SongRequest {
            song_id,
            song_name: format!("Song {song_id}"),
            requester: "tester".to_string(),
            timestamp: 1000.0 + song_id as f64,
            difficulty: Some(8.0),
            difficulty_tier: "extreme".to_string(),
        }
    }

    // ----------------- add -----------------

    #[test]
    fn add_returns_true_for_new_song() {
        let queue = SongQueue::new(10, false);
        assert!(queue.add(1, "A".into(), "u1".into(), Some(8.0), "extreme".into()));
        assert_eq!(queue.list().len(), 1);
    }

    #[test]
    fn add_rejects_duplicate_when_duplicates_disabled() {
        let queue = SongQueue::new(10, false);
        assert!(queue.add(1, "A".into(), "u1".into(), Some(8.0), "extreme".into()));
        assert!(!queue.add(1, "A again".into(), "u2".into(), Some(9.0), "extreme".into()));
        assert_eq!(queue.list().len(), 1);
    }

    #[test]
    fn add_allows_duplicate_when_duplicates_enabled() {
        let queue = SongQueue::new(10, true);
        assert!(queue.add(1, "A".into(), "u1".into(), Some(8.0), "extreme".into()));
        assert!(queue.add(1, "A again".into(), "u2".into(), Some(9.0), "extreme".into()));
        assert_eq!(queue.list().len(), 2);
    }

    #[test]
    fn add_rejects_when_queue_full() {
        let queue = SongQueue::new(2, false);
        assert!(queue.add(1, "A".into(), "u".into(), None, "extreme".into()));
        assert!(queue.add(2, "B".into(), "u".into(), None, "extreme".into()));
        assert!(!queue.add(3, "C".into(), "u".into(), None, "extreme".into()));
        assert_eq!(queue.list().len(), 2);
    }

    #[test]
    fn add_with_none_difficulty_is_stored() {
        let queue = SongQueue::new(10, false);
        assert!(queue.add(1, "A".into(), "u".into(), None, "extreme".into()));
        let item = &queue.list()[0];
        assert_eq!(item.difficulty, None);
    }

    // ----------------- next / complete / requeue_front -----------------

    #[test]
    fn next_returns_fifo_order() {
        let queue = SongQueue::new(10, false);
        queue.add(1, "A".into(), "u".into(), None, "extreme".into());
        queue.add(2, "B".into(), "u".into(), None, "extreme".into());
        let first = queue.next();
        assert_eq!(first.unwrap().song_id, 1);
        let second = queue.next();
        assert_eq!(second.unwrap().song_id, 2);
        assert!(queue.next().is_none());
    }

    #[test]
    fn next_on_empty_returns_none() {
        let queue = SongQueue::new(10, false);
        assert!(queue.next().is_none());
    }

    #[test]
    fn complete_writes_to_history() {
        let queue = SongQueue::new(10, false);
        let req = make_request(1);
        queue.complete(req.clone());
        let history = queue.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].song_id, 1);
    }

    #[test]
    fn requeue_front_puts_song_at_head() {
        let queue = SongQueue::new(10, false);
        queue.add(1, "A".into(), "u".into(), None, "extreme".into());
        queue.add(2, "B".into(), "u".into(), None, "extreme".into());
        // 模拟切歌失败：把第 1 首放回队首
        let first = queue.next().unwrap();
        queue.requeue_front(first);
        assert_eq!(queue.list()[0].song_id, 1);
        assert_eq!(queue.list()[1].song_id, 2);
    }

    #[test]
    fn next_does_not_auto_complete() {
        // next() 只出队，不写历史 —— 调用方需显式 complete()
        let queue = SongQueue::new(10, false);
        queue.add(1, "A".into(), "u".into(), None, "extreme".into());
        let _ = queue.next();
        assert!(queue.history().is_empty());
    }

    // ----------------- remove -----------------

    #[test]
    fn remove_existing_song_succeeds() {
        let queue = SongQueue::new(10, false);
        queue.add(1, "A".into(), "u".into(), None, "extreme".into());
        queue.add(2, "B".into(), "u".into(), None, "extreme".into());
        assert!(queue.remove(1));
        assert_eq!(queue.list().len(), 1);
        assert_eq!(queue.list()[0].song_id, 2);
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let queue = SongQueue::new(10, false);
        queue.add(1, "A".into(), "u".into(), None, "extreme".into());
        assert!(!queue.remove(999));
        assert_eq!(queue.list().len(), 1);
    }

    #[test]
    fn remove_from_empty_queue_returns_false() {
        let queue = SongQueue::new(10, false);
        assert!(!queue.remove(1));
    }

    // ----------------- clear -----------------

    #[test]
    fn clear_empties_queue_but_not_history() {
        let queue = SongQueue::new(10, false);
        queue.add(1, "A".into(), "u".into(), None, "extreme".into());
        queue.add(2, "B".into(), "u".into(), None, "extreme".into());
        queue.complete(make_request(0));
        queue.clear();
        assert!(queue.list().is_empty());
        assert_eq!(queue.history().len(), 1);
    }

    // ----------------- snapshot -----------------

    #[test]
    fn snapshot_reflects_current_state() {
        let queue = SongQueue::new(50, false);
        queue.add(1, "A".into(), "u".into(), Some(8.0), "extreme".into());
        queue.add(2, "B".into(), "u".into(), Some(9.0), "exextreme".into());
        let snap = queue.snapshot();
        assert_eq!(snap.size, 2);
        assert_eq!(snap.max_size, 50);
        assert!(!snap.allow_duplicates);
        assert_eq!(snap.songs.len(), 2);
        assert_eq!(snap.songs[0].position, 1);
        assert_eq!(snap.songs[1].position, 2);
        assert_eq!(snap.songs[0].request.song_id, 1);
    }

    #[test]
    fn snapshot_history_is_reversed_and_capped_at_5() {
        let queue = SongQueue::new(50, false);
        for i in 0..7 {
            queue.complete(make_request(i));
        }
        let snap = queue.snapshot();
        assert_eq!(snap.history.len(), 5);
        // 最新的在前
        assert_eq!(snap.history[0].song_id, 6);
        assert_eq!(snap.history[4].song_id, 2);
    }

    #[test]
    fn snapshot_empty_queue() {
        let queue = SongQueue::new(10, true);
        let snap = queue.snapshot();
        assert_eq!(snap.size, 0);
        assert!(snap.songs.is_empty());
        assert!(snap.history.is_empty());
        assert!(snap.allow_duplicates);
    }
}
