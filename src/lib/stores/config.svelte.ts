// Config state management using Svelte 5 runes
import type { Config, StorageMode, Theme } from '$lib/types';
import * as configApi from '$lib/tauri/config';

// Reactive state
let config = $state<Config>({
  storage: { mode: 'secure' },
  chat_display: {
    message_font_size: 13,
    show_timestamps: true,
    auto_scroll_enabled: true
  },
  ui: {
    theme: 'dark'
  }
});
let isLoaded = $state(false);
let error = $state<string | null>(null);

// Actions
async function load(): Promise<Config> {
  try {
    config = await configApi.configLoad();
    applyTheme(config.ui.theme);
    isLoaded = true;
    error = null;
    return config;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    // Use defaults on error
    applyTheme(config.ui.theme);
    isLoaded = true;
    return config;
  }
}

async function save(): Promise<void> {
  try {
    await configApi.configSave(config);
    error = null;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
    // Continue even if save fails
  }
}

async function setStorageMode(mode: StorageMode): Promise<void> {
  config.storage.mode = mode;
  try {
    await configApi.configSetValue('storage', 'mode', mode);
    error = null;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

async function setMessageFontSize(size: number): Promise<void> {
  // Validate range (10-24)
  const clampedSize = Math.max(10, Math.min(24, size));
  config.chat_display.message_font_size = clampedSize;
  try {
    await configApi.configSetValue('chat_display', 'message_font_size', clampedSize);
    error = null;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

async function setShowTimestamps(show: boolean): Promise<void> {
  config.chat_display.show_timestamps = show;
  try {
    await configApi.configSetValue('chat_display', 'show_timestamps', show);
    error = null;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

async function setAutoScrollEnabled(enabled: boolean): Promise<void> {
  config.chat_display.auto_scroll_enabled = enabled;
  try {
    await configApi.configSetValue('chat_display', 'auto_scroll_enabled', enabled);
    error = null;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

async function setTheme(theme: Theme): Promise<void> {
  config.ui.theme = theme;
  applyTheme(theme);
  try {
    await configApi.configSetValue('ui', 'theme', theme);
    error = null;
  } catch (e) {
    error = e instanceof Error ? e.message : String(e);
  }
}

function applyTheme(theme: Theme): void {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', theme);
  }
}

// Export store interface
export const configStore = {
  // Getters (reactive)
  get config() {
    return config;
  },
  get storageMode() {
    return config.storage.mode;
  },
  get messageFontSize() {
    return config.chat_display.message_font_size;
  },
  get showTimestamps() {
    return config.chat_display.show_timestamps;
  },
  get autoScrollEnabled() {
    return config.chat_display.auto_scroll_enabled;
  },
  get theme() {
    return config.ui.theme;
  },
  get isLoaded() {
    return isLoaded;
  },
  get error() {
    return error;
  },

  // Actions
  load,
  save,
  setStorageMode,
  setMessageFontSize,
  setShowTimestamps,
  setAutoScrollEnabled,
  setTheme
};
