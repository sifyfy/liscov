// Configuration types (09_config.md)

// Rust 型（commands/config.rs）から ts-rs で生成した型を re-export する
export type { StorageMode } from './generated/StorageMode';
export type { Theme } from './generated/Theme';
// 最後に選んだチャットモード（02_chat.md チャットモード）
export type { ChatModeSetting } from './generated/ChatModeSetting';
export type { StorageConfig } from './generated/StorageConfig';
export type { ChatDisplayConfig } from './generated/ChatDisplayConfig';
// チャットの取り方（02_chat.md「プッシュ通知」）
export type { ChatFetchConfig } from './generated/ChatFetchConfig';
export type { UiConfig } from './generated/UiConfig';
export type { Config } from './generated/Config';
// 生レスポンス保存設定（05_raw_response.md）
export type { SaveConfig } from './generated/SaveConfig';

import type { Config } from './generated/Config';

// Default values
export const DEFAULT_CONFIG: Config = {
  storage: {
    mode: 'secure'
  },
  chat_display: {
    message_font_size: 13,
    show_timestamps: true,
    hide_panel_on_connect: true,
    chat_mode: 'top',
    flowed_newcomers_height_ratio: 0.125,
    flowed_supporters_height_ratio: 0.125
  },
  chat_fetch: {
    push_notification: true
  },
  ui: {
    theme: 'dark'
  }
};
