// Config state management using Svelte 5 runes
import type { ChatModeSetting, Config, StorageMode, Theme } from '$lib/types';
import { DEFAULT_CONFIG } from '$lib/types';
import { normalizeError } from '$lib/tauri/errors';
import * as configApi from '$lib/tauri/config';

/** 高さを変えられる流れたものの欄 (02_chat.md「流れたもの」) */
export type FlowedSectionKind = 'newcomers' | 'supporters';
const FLOWED_HEIGHT_KEYS = {
  newcomers: 'flowed_newcomers_height_ratio',
  supporters: 'flowed_supporters_height_ratio',
} as const satisfies Record<FlowedSectionKind, keyof Config['chat_display']>;

// ファクトリ関数：テスト時に独立したストアインスタンスを生成できる
function createConfigStore() {
  // リアクティブ状態（ストアは config をミューテートするため、共有定数は deep copy して使う）
  let config = $state<Config>(structuredClone(DEFAULT_CONFIG));
  let isLoaded = $state(false);
  let error = $state<string | null>(null);

  // アクション
  async function load(): Promise<Config> {
    try {
      config = await configApi.configLoad();
      applyTheme(config.ui.theme);
      isLoaded = true;
      error = null;
      return config;
    } catch (e) {
      error = normalizeError(e).message;
      // エラー時はデフォルト値を使用
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
      error = normalizeError(e).message;
      // 保存失敗しても継続
    }
  }

  async function setStorageMode(mode: StorageMode): Promise<void> {
    config.storage.mode = mode;
    try {
      await configApi.configSetValue('storage', 'mode', mode);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  async function setMessageFontSize(size: number): Promise<void> {
    // 範囲バリデーション (10-24)
    const clampedSize = Math.max(10, Math.min(24, size));
    config.chat_display.message_font_size = clampedSize;
    try {
      await configApi.configSetValue('chat_display', 'message_font_size', clampedSize);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  async function setShowTimestamps(show: boolean): Promise<void> {
    config.chat_display.show_timestamps = show;
    try {
      await configApi.configSetValue('chat_display', 'show_timestamps', show);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  // 配信に接続したら操作パネルを隠す (09_config.md、02_chat.md「上の行と操作パネル」)
  async function setHidePanelOnConnect(enabled: boolean): Promise<void> {
    config.chat_display.hide_panel_on_connect = enabled;
    try {
      await configApi.configSetValue('chat_display', 'hide_panel_on_connect', enabled);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  // 流れたものの欄の高さの上限 (09_config.md、02_chat.md「流れたもの」)。範囲は呼ぶ側 (flowed-height.ts) で収めてある
  async function setFlowedHeightRatio(section: FlowedSectionKind, ratio: number): Promise<void> {
    const key = FLOWED_HEIGHT_KEYS[section];
    config.chat_display[key] = ratio;
    try {
      await configApi.configSetValue('chat_display', key, ratio);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  async function setChatMode(mode: ChatModeSetting): Promise<void> {
    config.chat_display.chat_mode = mode;
    try {
      await configApi.configSetValue('chat_display', 'chat_mode', mode);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  async function setTheme(theme: Theme): Promise<void> {
    config.ui.theme = theme;
    applyTheme(theme);
    try {
      await configApi.configSetValue('ui', 'theme', theme);
      error = null;
    } catch (e) {
      error = normalizeError(e).message;
    }
  }

  function applyTheme(theme: Theme): void {
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('data-theme', theme);
    }
  }

  return {
    // Getters (リアクティブ)
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
    get hidePanelOnConnect() {
      return config.chat_display.hide_panel_on_connect;
    },
    get flowedNewcomersHeightRatio() {
      return config.chat_display.flowed_newcomers_height_ratio;
    },
    get flowedSupportersHeightRatio() {
      return config.chat_display.flowed_supporters_height_ratio;
    },
    get chatMode() {
      return config.chat_display.chat_mode;
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

    // アクション
    load,
    save,
    setStorageMode,
    setMessageFontSize,
    setShowTimestamps,
    setHidePanelOnConnect,
    setFlowedHeightRatio,
    setChatMode,
    setTheme
  };
}

// アプリ全体で使うシングルトンインスタンス
export const configStore = createConfigStore();
