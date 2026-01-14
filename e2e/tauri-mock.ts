import { Page } from '@playwright/test';

// Mock responses for Tauri commands (aligned with latest specification)
const mockResponses: Record<string, unknown> = {
  // Chat commands (02_chat.md)
  connect_to_stream: {
    success: true,
    stream_title: 'Test Stream',
    broadcaster_channel_id: 'UC_test_channel_123',
    broadcaster_name: 'Test Broadcaster',
    is_replay: false,
    error: null,
    session_id: 'test-session-123',
  },
  disconnect_stream: null,
  get_chat_messages: [],
  set_chat_mode: true,

  // WebSocket commands (03_websocket.md)
  websocket_start: { actual_port: 8765 },
  websocket_stop: null,
  websocket_get_status: {
    is_running: false,
    actual_port: null,
    connected_clients: 0,
  },

  // Database commands (08_database.md)
  session_get_list: [],
  session_get_messages: [],
  session_create: 'test-session-123',
  session_end: null,

  // Viewer commands (06_viewer.md)
  viewer_get_list: [],
  viewer_upsert_custom_info: 1,
  viewer_get_profile: null,
  viewer_search: [],
  viewer_delete: true,
  broadcaster_get_list: [],
  broadcaster_delete: [true, 0],
  get_viewer_custom_info: null,
  get_all_viewer_custom_info: {},
  delete_viewer_custom_info: true,

  // Analytics commands (07_revenue.md) - tier-based statistics
  get_revenue_analytics: {
    super_chat_count: 0,
    super_chat_by_tier: {
      tier_red: 0,
      tier_magenta: 0,
      tier_orange: 0,
      tier_yellow: 0,
      tier_green: 0,
      tier_cyan: 0,
      tier_blue: 0,
    },
    super_sticker_count: 0,
    membership_gains: 0,
    hourly_stats: [],
    top_contributors: [],
  },
  get_session_analytics: null,
  export_session_data: null,
  export_current_messages: null,

  // TTS commands (04_tts.md)
  tts_get_config: {
    enabled: false,
    backend: 'none',
    read_author_name: true,
    add_honorific: true,
    strip_at_prefix: true,
    strip_handle_suffix: true,
    read_superchat_amount: true,
    max_text_length: 200,
    queue_size_limit: 50,
    bouyomichan: {
      host: 'localhost',
      port: 50080,
      voice: 0,
      volume: -1,
      speed: -1,
      tone: -1,
    },
    voicevox: {
      host: 'localhost',
      port: 50021,
      speaker_id: 1,
      volume_scale: 1.0,
      speed_scale: 1.0,
      pitch_scale: 0.0,
      intonation_scale: 1.0,
    },
  },
  tts_update_config: null,
  tts_test_connection: true,
  tts_start: null,
  tts_stop: null,
  tts_clear_queue: null,
  tts_get_status: { is_processing: false, queue_size: 0 },
  tts_speak_direct: null,

  // Auth commands (01_auth.md)
  auth_get_status: {
    is_authenticated: false,
    has_saved_credentials: false,
    storage_type: 'secure',
    storage_error: null,
  },
  auth_load_credentials: false,
  auth_save_raw_cookies: null,
  auth_save_credentials: null,
  auth_delete_credentials: null,
  auth_validate_credentials: true,
  auth_open_window: null,
  auth_check_session_validity: { is_valid: false, checked_at: null, error: null },
  auth_use_fallback_storage: true,

  // Raw response save commands (05_raw_response.md)
  raw_response_get_config: {
    enabled: false,
    file_path: 'raw_responses.ndjson',
    max_file_size_mb: 100,
    enable_rotation: true,
    max_backup_files: 5,
  },
  raw_response_update_config: null,
  raw_response_resolve_path: 'C:\\Users\\test\\AppData\\Roaming\\liscov\\raw_responses.ndjson',

  // Legacy aliases for backward compatibility
  get_save_config: {
    enabled: false,
    file_path: 'raw_responses.ndjson',
    max_file_size_mb: 100,
    enable_rotation: true,
    max_backup_files: 5,
  },
  update_save_config: null,
  resolve_save_path: 'C:\\Users\\test\\AppData\\Roaming\\liscov\\raw_responses.ndjson',

  // Config commands (09_config.md)
  config_load: {
    storage: { mode: 'secure' },
    chat_display: {
      message_font_size: 13,
      show_timestamps: true,
      auto_scroll_enabled: true,
    },
  },
  config_save: null,
  config_get_value: null,
  config_set_value: null,
};

// State management for mock
let mockState = {
  websocketRunning: false,
  websocketPort: null as number | null,
  isConnected: false,
  streamTitle: '',
};

export async function setupTauriMock(page: Page) {
  await page.addInitScript(() => {
    // Initialize command tracking array
    // @ts-expect-error - tracking
    window.__INVOKED_COMMANDS__ = [];

    // @ts-expect-error - Tauri mock
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd: string, args?: unknown) => {
        console.log('[Tauri Mock] invoke:', cmd, args);

        // Track invoked commands with their arguments
        // @ts-expect-error - tracking
        window.__INVOKED_COMMANDS__.push({ cmd, args });

        // Handle stateful commands
        if (cmd === 'websocket_start') {
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.websocketRunning = true;
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.websocketPort = 8765;
          return { actual_port: 8765 };
        }

        if (cmd === 'websocket_stop') {
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.websocketRunning = false;
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.websocketPort = null;
          return null;
        }

        if (cmd === 'websocket_get_status') {
          return {
            // @ts-expect-error - mock state
            is_running: window.__MOCK_STATE__.websocketRunning,
            // @ts-expect-error - mock state
            actual_port: window.__MOCK_STATE__.websocketPort,
            connected_clients: 0,
          };
        }

        if (cmd === 'connect_to_stream') {
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.isConnected = true;
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.streamTitle = 'Test Stream';
          return {
            stream_title: 'Test Stream',
            broadcaster_channel_id: 'UC_test_channel_123',
            video_id: 'test_video_id',
          };
        }

        if (cmd === 'disconnect_stream') {
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.isConnected = false;
          // @ts-expect-error - mock state
          window.__MOCK_STATE__.streamTitle = '';
          return null;
        }

        // Return mock response for other commands
        // @ts-expect-error - mock responses
        const response = window.__MOCK_RESPONSES__[cmd];
        if (response !== undefined) {
          return response;
        }

        console.warn('[Tauri Mock] Unknown command:', cmd);
        return null;
      },
    };

    // Initialize mock state
    // @ts-expect-error - mock state
    window.__MOCK_STATE__ = {
      websocketRunning: false,
      websocketPort: null,
      isConnected: false,
      streamTitle: '',
    };

    // Store mock responses (aligned with latest specification)
    // @ts-expect-error - mock responses
    window.__MOCK_RESPONSES__ = {
      // Chat
      get_chat_messages: [],
      set_chat_mode: true,

      // Sessions (08_database.md)
      session_get_list: [],
      session_get_messages: [],

      // Viewers (06_viewer.md)
      viewer_get_list: [],
      broadcaster_get_list: [],
      get_all_viewer_custom_info: {},

      // Revenue analytics (07_revenue.md) - tier-based
      get_revenue_analytics: {
        super_chat_count: 0,
        super_chat_by_tier: {
          tier_red: 0,
          tier_magenta: 0,
          tier_orange: 0,
          tier_yellow: 0,
          tier_green: 0,
          tier_cyan: 0,
          tier_blue: 0,
        },
        super_sticker_count: 0,
        membership_gains: 0,
        hourly_stats: [],
        top_contributors: [],
      },

      // TTS (04_tts.md)
      tts_get_config: {
        enabled: false,
        backend: 'none',
        read_author_name: true,
        add_honorific: true,
        strip_at_prefix: true,
        strip_handle_suffix: true,
        read_superchat_amount: true,
        max_text_length: 200,
        queue_size_limit: 50,
        bouyomichan: {
          host: 'localhost',
          port: 50080,
          voice: 0,
          volume: -1,
          speed: -1,
          tone: -1,
        },
        voicevox: {
          host: 'localhost',
          port: 50021,
          speaker_id: 1,
          volume_scale: 1.0,
          speed_scale: 1.0,
          pitch_scale: 0.0,
          intonation_scale: 1.0,
        },
      },
      tts_get_status: { is_processing: false, queue_size: 0 },
      tts_speak_direct: null,
      tts_test_connection: true,

      // Auth (01_auth.md)
      auth_get_status: {
        is_authenticated: false,
        has_saved_credentials: false,
        storage_type: 'secure',
        storage_error: null,
      },
      auth_open_window: null,
      auth_check_session_validity: { is_valid: false, checked_at: null, error: null },
      auth_use_fallback_storage: true,

      // Raw response save (05_raw_response.md)
      raw_response_get_config: {
        enabled: false,
        file_path: 'raw_responses.ndjson',
        max_file_size_mb: 100,
        enable_rotation: true,
        max_backup_files: 5,
      },
      raw_response_update_config: null,
      raw_response_resolve_path: 'C:\\Users\\test\\AppData\\Roaming\\liscov\\raw_responses.ndjson',

      // Legacy aliases
      get_save_config: {
        enabled: false,
        file_path: 'raw_responses.ndjson',
        max_file_size_mb: 100,
        enable_rotation: true,
        max_backup_files: 5,
      },
      update_save_config: null,
      resolve_save_path: 'C:\\Users\\test\\AppData\\Roaming\\liscov\\raw_responses.ndjson',

      // Config (09_config.md)
      config_load: {
        storage: { mode: 'secure' },
        chat_display: {
          message_font_size: 13,
          show_timestamps: true,
          auto_scroll_enabled: true,
        },
      },
      config_set_value: null,
    };

    // Mock Tauri event listener
    // @ts-expect-error - Tauri mock
    window.__TAURI_INTERNALS__.transformCallback = (callback: unknown) => {
      return callback;
    };
  });
}

export function resetMockState() {
  mockState = {
    websocketRunning: false,
    websocketPort: null,
    isConnected: false,
    streamTitle: '',
  };
}
