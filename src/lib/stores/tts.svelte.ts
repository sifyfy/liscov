// TTS store

import type { TtsConfig, TtsStatus, TtsPriority, TtsLaunchStatus } from '$lib/types';
import { defaultTtsConfig } from '$lib/types';
import * as ttsApi from '$lib/tauri/tts';

interface TtsStore {
  config: TtsConfig;
  status: TtsStatus;
  launchStatus: TtsLaunchStatus;
  isLoading: boolean;
  error: string | null;
  connectionTestResult: boolean | null;
  testingBackend: string | null;
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === 'string') {
    return error;
  }

  if (error && typeof error === 'object') {
    const record = error as Record<string, unknown>;
    for (const key of ['message', 'error', 'details']) {
      const value = record[key];
      if (typeof value === 'string' && value.length > 0) {
        return value;
      }
    }

    try {
      return JSON.stringify(error);
    } catch {
      // JSON変換に失敗した場合は最後に String() にフォールバックする
    }
  }

  return String(error);
}

function createTtsStore() {
  let config = $state<TtsConfig>({ ...defaultTtsConfig });
  let status = $state<TtsStatus>({
    is_processing: false,
    queue_size: 0,
    backend_name: null
  });
  let launchStatus = $state<TtsLaunchStatus>({
    bouyomichan_launched: false,
    voicevox_launched: false
  });
  let isLoading = $state(false);
  let error = $state<string | null>(null);
  let connectionTestResult = $state<boolean | null>(null);
  let testingBackend = $state<string | null>(null);

  return {
    get config() {
      return config;
    },
    get status() {
      return status;
    },
    get launchStatus() {
      return launchStatus;
    },
    get isLoading() {
      return isLoading;
    },
    get error() {
      return error;
    },
    get connectionTestResult() {
      return connectionTestResult;
    },
    get testingBackend() {
      return testingBackend;
    },

    async loadConfig() {
      isLoading = true;
      error = null;
      try {
        config = await ttsApi.ttsGetConfig();
      } catch (e) {
        error = toErrorMessage(e);
      } finally {
        isLoading = false;
      }
    },

    async saveConfig(newConfig: TtsConfig) {
      isLoading = true;
      error = null;
      try {
        await ttsApi.ttsUpdateConfig(newConfig);
        config = newConfig;
      } catch (e) {
        error = toErrorMessage(e);
      } finally {
        isLoading = false;
      }
    },

    async updateConfig(updates: Partial<TtsConfig>) {
      const newConfig = { ...config, ...updates };
      await this.saveConfig(newConfig);
    },

    async testConnection(backend?: string) {
      testingBackend = backend || config.backend;
      connectionTestResult = null;
      error = null;
      try {
        connectionTestResult = await ttsApi.ttsTestConnection(backend);
      } catch (e) {
        error = toErrorMessage(e);
        connectionTestResult = false;
      } finally {
        testingBackend = null;
      }
      return connectionTestResult;
    },

    async speak(text: string, options?: { priority?: TtsPriority; authorName?: string; amount?: string }) {
      try {
        await ttsApi.ttsSpeak(text, options);
      } catch (e) {
        error = toErrorMessage(e);
      }
    },

    async speakDirect(text: string) {
      try {
        await ttsApi.ttsSpeakDirect(text);
      } catch (e) {
        error = toErrorMessage(e);
      }
    },

    async start() {
      error = null;
      try {
        await ttsApi.ttsStart();
        await this.refreshStatus();
      } catch (e) {
        error = toErrorMessage(e);
      }
    },

    async stop() {
      error = null;
      try {
        await ttsApi.ttsStop();
        await this.refreshStatus();
      } catch (e) {
        error = toErrorMessage(e);
      }
    },

    async clearQueue() {
      error = null;
      try {
        await ttsApi.ttsClearQueue();
        await this.refreshStatus();
      } catch (e) {
        error = toErrorMessage(e);
      }
    },

    async refreshStatus() {
      try {
        status = await ttsApi.ttsGetStatus();
      } catch (e) {
        error = toErrorMessage(e);
      }
    },

    clearError() {
      error = null;
    },

    clearTestResult() {
      connectionTestResult = null;
    },

    async discoverExe(backend: string): Promise<string | null> {
      try {
        return await ttsApi.ttsDiscoverExe(backend);
      } catch (e) {
        error = toErrorMessage(e);
        return null;
      }
    },

    async selectExe(): Promise<string | null> {
      try {
        return await ttsApi.ttsSelectExe();
      } catch (e) {
        error = toErrorMessage(e);
        return null;
      }
    },

    async launchBackend(backend: string, exePath?: string): Promise<number | null> {
      error = null;
      try {
        const pid = await ttsApi.ttsLaunchBackend(backend, exePath);
        await this.refreshLaunchStatus();
        return pid;
      } catch (e) {
        error = toErrorMessage(e);
        return null;
      }
    },

    async killBackend(backend: string): Promise<boolean> {
      error = null;
      try {
        await ttsApi.ttsKillBackend(backend);
        await this.refreshLaunchStatus();
        return true;
      } catch (e) {
        error = toErrorMessage(e);
        return false;
      }
    },

    async refreshLaunchStatus() {
      try {
        launchStatus = await ttsApi.ttsGetLaunchStatus();
      } catch (e) {
        error = toErrorMessage(e);
      }
    }
  };
}

export const ttsStore = createTtsStore();
