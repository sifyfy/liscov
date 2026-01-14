// TTS types

export type TtsBackend = 'none' | 'bouyomichan' | 'voicevox';
export type TtsPriority = 'normal' | 'membership' | 'superchat';

export interface TtsConfig {
  enabled: boolean;
  backend: TtsBackend;
  read_author_name: boolean;
  add_honorific: boolean;
  strip_at_prefix: boolean;
  strip_handle_suffix: boolean;
  read_superchat_amount: boolean;
  max_text_length: number;
  queue_size_limit: number;
  // Bouyomichan settings
  bouyomichan_host: string;
  bouyomichan_port: number;
  bouyomichan_voice: number;
  bouyomichan_volume: number;
  bouyomichan_speed: number;
  bouyomichan_tone: number;
  // VOICEVOX settings
  voicevox_host: string;
  voicevox_port: number;
  voicevox_speaker_id: number;
  voicevox_volume_scale: number;
  voicevox_speed_scale: number;
  voicevox_pitch_scale: number;
  voicevox_intonation_scale: number;
}

export interface TtsStatus {
  is_processing: boolean;
  queue_size: number;
  backend_name: string | null;
}

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
  bouyomichan_host: 'localhost',
  bouyomichan_port: 50080,
  bouyomichan_voice: 0,
  bouyomichan_volume: -1,
  bouyomichan_speed: -1,
  bouyomichan_tone: -1,
  voicevox_host: 'localhost',
  voicevox_port: 50021,
  voicevox_speaker_id: 1,
  voicevox_volume_scale: 1.0,
  voicevox_speed_scale: 1.0,
  voicevox_pitch_scale: 0.0,
  voicevox_intonation_scale: 1.0
};
