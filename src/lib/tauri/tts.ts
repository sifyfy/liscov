// Tauri TTS API wrapper

import { invoke } from '@tauri-apps/api/core';
import type { TtsConfig, TtsPriority, TtsStatus, TtsLaunchStatus } from '$lib/types';

export interface SpeakOptions {
  priority?: TtsPriority;
  authorName?: string;
  amount?: string;
}

export async function ttsSpeak(text: string, options?: SpeakOptions): Promise<void> {
  await invoke('tts_speak', {
    text,
    priority: options?.priority,
    author_name: options?.authorName,
    amount: options?.amount
  });
}

export async function ttsSpeakDirect(text: string): Promise<void> {
  await invoke('tts_speak_direct', { text });
}

export async function ttsUpdateConfig(config: TtsConfig): Promise<void> {
  await invoke('tts_update_config', { config });
}

export async function ttsGetConfig(): Promise<TtsConfig> {
  return invoke('tts_get_config');
}

export async function ttsTestConnection(backend?: string): Promise<boolean> {
  return invoke('tts_test_connection', { backend });
}

export async function ttsStart(): Promise<void> {
  await invoke('tts_start');
}

export async function ttsStop(): Promise<void> {
  await invoke('tts_stop');
}

export async function ttsClearQueue(): Promise<void> {
  await invoke('tts_clear_queue');
}

export async function ttsGetStatus(): Promise<TtsStatus> {
  return invoke('tts_get_status');
}

export async function ttsDiscoverExe(backend: string): Promise<string | null> {
  return invoke('tts_discover_exe', { backend });
}

export async function ttsSelectExe(): Promise<string | null> {
  return invoke('tts_select_exe');
}

export async function ttsLaunchBackend(backend: string, exePath?: string): Promise<number> {
  // Tauri v2 converts camelCase args to snake_case for Rust commands
  return invoke('tts_launch_backend', { backend, exePath });
}

export async function ttsKillBackend(backend: string): Promise<void> {
  await invoke('tts_kill_backend', { backend });
}

export async function ttsGetLaunchStatus(): Promise<TtsLaunchStatus> {
  return invoke('tts_get_launch_status');
}
