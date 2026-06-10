// TTS store

import type { TtsConfig, TtsStatus, TtsPriority, TtsLaunchStatus } from '$lib/types';
import { defaultTtsConfig } from '$lib/types';
import * as ttsApi from '$lib/tauri/tts';
import { normalizeError } from '$lib/tauri/errors';


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
        error = normalizeError(e).message;
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
        error = normalizeError(e).message;
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
        error = normalizeError(e).message;
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
        error = normalizeError(e).message;
      }
    },

    async speakDirect(text: string) {
      try {
        await ttsApi.ttsSpeakDirect(text);
      } catch (e) {
        error = normalizeError(e).message;
      }
    },

    async start() {
      error = null;
      try {
        await ttsApi.ttsStart();
        await this.refreshStatus();
      } catch (e) {
        error = normalizeError(e).message;
      }
    },

    async stop() {
      error = null;
      try {
        await ttsApi.ttsStop();
        await this.refreshStatus();
      } catch (e) {
        error = normalizeError(e).message;
      }
    },

    async clearQueue() {
      error = null;
      try {
        await ttsApi.ttsClearQueue();
        await this.refreshStatus();
      } catch (e) {
        error = normalizeError(e).message;
      }
    },

    async refreshStatus() {
      try {
        status = await ttsApi.ttsGetStatus();
      } catch (e) {
        error = normalizeError(e).message;
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
        error = normalizeError(e).message;
        return null;
      }
    },

    async selectExe(): Promise<string | null> {
      try {
        return await ttsApi.ttsSelectExe();
      } catch (e) {
        error = normalizeError(e).message;
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
        error = normalizeError(e).message;
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
        error = normalizeError(e).message;
        return false;
      }
    },

    async refreshLaunchStatus() {
      try {
        launchStatus = await ttsApi.ttsGetLaunchStatus();
      } catch (e) {
        error = normalizeError(e).message;
      }
    }
  };
}

export const ttsStore = createTtsStore();
