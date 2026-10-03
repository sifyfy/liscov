// TTS types

// Rust 型（commands/tts.rs・tts/config.rs）から ts-rs で生成した型を re-export する
export type { TtsBackend } from './generated/TtsBackend';
export type { TtsConfig } from './generated/TtsConfig';
export type { TtsStatus } from './generated/TtsStatus';
export type { TtsLaunchStatus } from './generated/TtsLaunchStatus';

import type { TtsConfig } from './generated/TtsConfig';

// 読み上げの優先度（tts_speak の priority。Rust 側は文字列で受けて parse_tts_priority で解釈する）
export type TtsPriority = 'normal' | 'membership' | 'superchat';

export const defaultTtsConfig: TtsConfig = {
  enabled: false,
  backend: 'none',
  read_author_name: true,
  add_honorific: true,
  strip_at_prefix: true,
  strip_handle_suffix: true,
  read_superchat_amount: true,
  max_text_length: 200,
  queue_size_limit: 50,
  first_comment_prefix_enabled: false,
  first_comment_prefix: '',
  first_comment_only: false,
  bouyomichan_host: 'localhost',
  bouyomichan_port: 50080,
  bouyomichan_voice: 0,
  bouyomichan_volume: -1,
  bouyomichan_speed: -1,
  bouyomichan_tone: -1,
  bouyomichan_auto_launch: false,
  bouyomichan_exe_path: null,
  bouyomichan_auto_close: true,
  voicevox_host: 'localhost',
  voicevox_port: 50021,
  voicevox_speaker_id: 1,
  voicevox_volume_scale: 1.0,
  voicevox_speed_scale: 1.0,
  voicevox_pitch_scale: 0.0,
  voicevox_intonation_scale: 1.0,
  voicevox_auto_launch: false,
  voicevox_exe_path: null,
  voicevox_auto_close: true
};
