// Configuration types (09_config.md)

export type StorageMode = 'secure' | 'fallback';

export type Theme = 'dark' | 'light';

export interface StorageConfig {
  mode: StorageMode;
}

export interface ChatDisplayConfig {
  message_font_size: number;
  show_timestamps: boolean;
  auto_scroll_enabled: boolean;
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
    auto_scroll_enabled: true
  },
  ui: {
    theme: 'dark'
  }
};
