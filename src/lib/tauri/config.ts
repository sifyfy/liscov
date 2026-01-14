// Tauri Config API wrapper

import { invoke } from '@tauri-apps/api/core';
import type { Config } from '$lib/types';

export async function configLoad(): Promise<Config> {
  return invoke('config_load');
}

export async function configSave(config: Config): Promise<void> {
  await invoke('config_save', { config });
}

export async function configGetValue<T>(section: string, key: string): Promise<T | null> {
  return invoke('config_get_value', { section, key });
}

export async function configSetValue<T>(section: string, key: string, value: T): Promise<void> {
  await invoke('config_set_value', { section, key, value });
}
