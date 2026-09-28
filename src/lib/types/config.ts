// Configuration types (09_config.md)

export type StorageMode = 'secure' | 'fallback';

export type Theme = 'dark' | 'light';

// 最後に選んだチャットモード（02_chat.md チャットモード）
export type ChatModeSetting = 'top' | 'all';

export interface StorageConfig {
  mode: StorageMode;
}

export interface ChatDisplayConfig {
  message_font_size: number;
  show_timestamps: boolean;
  auto_scroll_enabled: boolean;
  chat_mode: ChatModeSetting;
}

export interface UiConfig {
  theme: Theme;
}

export interface Config {
  storage: StorageConfig;
  chat_display: ChatDisplayConfig;
  ui: UiConfig;
}

// Default values
export const DEFAULT_CONFIG: Config = {
  storage: {
    mode: 'secure'
  },
  chat_display: {
    message_font_size: 13,
    show_timestamps: true,
    auto_scroll_enabled: true,
    chat_mode: 'top'
  },
  ui: {
    theme: 'dark'
  }
};
