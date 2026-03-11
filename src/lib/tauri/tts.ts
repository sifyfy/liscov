// TTS 関連の Tauri コマンドラッパー

import { invoke } from '@tauri-apps/api/core';
import type { TtsConfig, TtsPriority, TtsStatus, TtsLaunchStatus } from '$lib/types';
import { normalizeError } from './errors';

export interface SpeakOptions {
  priority?: TtsPriority;
  authorName?: string;
  amount?: string;
}

export async function ttsSpeak(text: string, options?: SpeakOptions): Promise<void> {
  try {
    await invoke('tts_speak', {
      text,
      priority: options?.priority,
      author_name: options?.authorName,
      amount: options?.amount
    });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsSpeakDirect(text: string): Promise<void> {
  try {
    await invoke('tts_speak_direct', { text });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsUpdateConfig(config: TtsConfig): Promise<void> {
  try {
    await invoke('tts_update_config', { config });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsGetConfig(): Promise<TtsConfig> {
  try {
    return await invoke('tts_get_config');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsTestConnection(backend?: string): Promise<boolean> {
  try {
    return await invoke('tts_test_connection', { backend });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsStart(): Promise<void> {
  try {
    await invoke('tts_start');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsStop(): Promise<void> {
  try {
    await invoke('tts_stop');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsClearQueue(): Promise<void> {
  try {
    await invoke('tts_clear_queue');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsGetStatus(): Promise<TtsStatus> {
  try {
    return await invoke('tts_get_status');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsDiscoverExe(backend: string): Promise<string | null> {
  try {
    return await invoke('tts_discover_exe', { backend });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsSelectExe(): Promise<string | null> {
  try {
    return await invoke('tts_select_exe');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsLaunchBackend(backend: string, exePath?: string): Promise<number> {
  try {
    // Tauri v2 は camelCase 引数を Rust コマンド向けに snake_case に変換する
    return await invoke('tts_launch_backend', { backend, exePath });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsKillBackend(backend: string): Promise<void> {
  try {
    await invoke('tts_kill_backend', { backend });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function ttsGetLaunchStatus(): Promise<TtsLaunchStatus> {
  try {
    return await invoke('tts_get_launch_status');
  } catch (e) {
    throw normalizeError(e);
  }
}
