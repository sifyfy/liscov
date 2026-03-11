// 設定関連の Tauri コマンドラッパー

import { invoke } from '@tauri-apps/api/core';
import type { Config } from '$lib/types';
import { normalizeError } from './errors';

export async function configLoad(): Promise<Config> {
  try {
    return await invoke('config_load');
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function configSave(config: Config): Promise<void> {
  try {
    await invoke('config_save', { config });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function configGetValue<T>(section: string, key: string): Promise<T | null> {
  try {
    return await invoke('config_get_value', { section, key });
  } catch (e) {
    throw normalizeError(e);
  }
}

export async function configSetValue<T>(section: string, key: string, value: T): Promise<void> {
  try {
    await invoke('config_set_value', { section, key, value });
  } catch (e) {
    throw normalizeError(e);
  }
}
