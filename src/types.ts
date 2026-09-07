export interface AppConfig {
  room_id: number;
  hotkey: string;
  data_dir: string;
  mods_dir: string;
  max_queue_size: number;
  allow_duplicates: boolean;
  obs_overlay_enabled: boolean;
  obs_overlay_host: string;
  obs_overlay_port: number;
  obs_overlay_title: string;
  song_command_prefix: string;
  sessdata: string;
  fetch_chinese_names: boolean;
  log_to_file: boolean;
  http_proxy: string;
  default_search_difficulty: string;
  difficulty_tolerance: number;
  difficulty_fallback: string;
  llm_enabled: boolean;
  llm_api_key: string;
  llm_base_url: string;
  llm_model: string;
  llm_max_tokens: number | null;
  config_file: string;
}

export interface GameInstallation {
  game_dir: string;
  mods_dir: string | null;
}

export interface SongRequest {
  song_id: number;
  song_name: string;
  requester: string;
  timestamp: number;
  difficulty: number | null;
  difficulty_tier: string;
}

export interface SongInfo {
  pv_id: number;
  name: string;
  name_en: string | null;
  name_zh: string | null;
  authors: string[];
  difficulty: Record<string, number>;
  source: string | null;
  mod_name: string | null;
  aliases: string[];
}

export interface SearchResult {
  pv_id: number;
  display_name: string;
  difficulty: number | null;
  difficulty_tier: string | null;
}

export interface SongRequestFailure {
  requester: string;
  query: string;
  message: string;
}

export interface DebugSongRequestResult {
  is_song_request: boolean;
  query: string;
  matched: boolean;
  song_id: number | null;
  song_name: string | null;
  added: boolean;
  requester: string | null;
  message: string;
}

export interface DanmakuEvent {
  user_name: string;
  content: string;
  is_song_request: boolean;
  timestamp: number;
}

export interface DanmakuStatus {
  connected: boolean;
  room_id: number;
}

export interface HotkeyStatus {
  registered: boolean;
  hotkey: string;
  message: string;
}

export interface OBSOverlayStatus {
  running: boolean;
  url: string;
  message: string;
}

export interface RebuildReport {
  data_dir: string;
  mods_dir: string | null;
  total: number;
  base_imported: number;
  mods_imported: number;
  mods_scanned: number;
  aliases_imported: number;
  chinese_names_merged: number;
  chinese_names_total: number;
  removed_mdata: number;
  removed_unnamed: number;
  sources: Record<string, number>;
}
