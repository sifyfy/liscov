<script lang="ts">
  import { ttsStore } from '$lib/stores';
  import type { TtsBackend, TtsConfig } from '$lib/types';
  import { onMount } from 'svelte';

  // Local state for editing
  let config = $state<TtsConfig | null>(null);
  let testText = $state('テスト読み上げです');
  let isSpeaking = $state(false);

  let isLaunching = $state<{ bouyomichan: boolean; voicevox: boolean }>({
    bouyomichan: false,
    voicevox: false
  });

  onMount(() => {
    loadConfig();
    ttsStore.refreshLaunchStatus();
  });

  async function loadConfig() {
    await ttsStore.loadConfig();
    config = { ...ttsStore.config };
  }

  async function discoverExe(backend: 'bouyomichan' | 'voicevox') {
    const path = await ttsStore.discoverExe(backend);
    if (path && config) {
      if (backend === 'bouyomichan') {
        config.bouyomichan_exe_path = path;
      } else {
        config.voicevox_exe_path = path;
      }
      await autoSave();
    }
  }

  async function browseExe(backend: 'bouyomichan' | 'voicevox') {
    const path = await ttsStore.selectExe();
    if (path && config) {
      if (backend === 'bouyomichan') {
        config.bouyomichan_exe_path = path;
      } else {
        config.voicevox_exe_path = path;
      }
      await autoSave();
    }
  }

  async function toggleLaunch(backend: 'bouyomichan' | 'voicevox') {
    if (!config) return;
    const isLaunched = backend === 'bouyomichan'
      ? ttsStore.launchStatus.bouyomichan_launched
      : ttsStore.launchStatus.voicevox_launched;
    const exePath = backend === 'bouyomichan'
      ? config.bouyomichan_exe_path ?? undefined
      : config.voicevox_exe_path ?? undefined;

    isLaunching[backend] = true;
    try {
      if (isLaunched) {
        await ttsStore.killBackend(backend);
      } else {
        await ttsStore.launchBackend(backend, exePath);
      }
    } finally {
      isLaunching[backend] = false;
    }
  }

  // Auto-save when config changes (debounced)
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  async function autoSave() {
    if (!config) return;

    // Clear any pending save
    if (saveTimeout) {
      clearTimeout(saveTimeout);
    }

    // Debounce save for 300ms
    saveTimeout = setTimeout(async () => {
      await ttsStore.saveConfig(config!);
    }, 300);
  }

  async function saveImmediately() {
    if (!config) return;

    if (saveTimeout) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }

    await ttsStore.saveConfig(config);
  }

  type ToggleConfigKey =
    | 'bouyomichan_auto_launch'
    | 'bouyomichan_auto_close'
    | 'voicevox_auto_launch'
    | 'voicevox_auto_close';

  async function toggleConfig(key: ToggleConfigKey) {
    if (!config) return;

    config = {
      ...config,
      [key]: !config[key]
    };

    await saveImmediately();
  }

  async function testConnection() {
    if (!config) return;
    await ttsStore.testConnection(config.backend);
  }

  async function testSpeak() {
    if (!testText || isSpeaking) return;
    isSpeaking = true;
    try {
      await ttsStore.speakDirect(testText);
    } finally {
      // Add a small delay to show the speaking state
      setTimeout(() => {
        isSpeaking = false;
      }, 500);
    }
  }

  async function toggleEnabled() {
    if (!config) return;
    config.enabled = !config.enabled;
    await ttsStore.saveConfig(config);
    if (config.enabled) {
      await ttsStore.start();
    } else {
      await ttsStore.stop();
    }
  }

  function handleBackendChange(e: Event) {
    if (!config) return;
    config.backend = (e.target as HTMLSelectElement).value as TtsBackend;
    autoSave();
  }

  // Watch for config changes and auto-save
  function handleConfigChange() {
    autoSave();
  }
</script>

<div class="p-6 space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold text-[var(--text-primary)]" style="font-family: var(--font-heading);">TTS設定</h2>
    <div class="flex items-center gap-4">
      <span class="text-sm text-[var(--text-muted)]">
        キュー: {ttsStore.status.queue_size}件
      </span>
      {#if ttsStore.status.is_processing}
        <span class="px-2 py-1 text-xs bg-[var(--success-subtle)] text-[var(--success)] rounded border border-[var(--border-default)]">処理中</span>
      {/if}
    </div>
  </div>

  {#if ttsStore.isLoading}
    <div class="text-center text-[var(--text-muted)]">読み込み中...</div>
  {:else if config}
    <!-- Enable/Disable Toggle -->
    <div class="flex items-center justify-between p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)]">
      <div>
        <h3 class="text-[var(--text-primary)] font-medium">TTS読み上げ</h3>
        <p class="text-sm text-[var(--text-secondary)]">チャットメッセージを音声で読み上げます</p>
      </div>
      <button
        onclick={toggleEnabled}
        aria-label={config.enabled ? 'TTSを無効にする' : 'TTSを有効にする'}
        class="{config.enabled ? 'bg-[var(--success)]' : 'bg-[var(--bg-surface-3)]'} relative inline-flex h-6 w-11 items-center rounded-full transition-colors"
      >
        <span
          class="{config.enabled ? 'translate-x-6' : 'translate-x-1'} inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow"
        ></span>
      </button>
    </div>

    <!-- Backend Selection -->
    <div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] space-y-4">
      <h3 class="text-[var(--text-primary)] font-medium">バックエンド設定</h3>

      <div>
        <label for="backend" class="block text-sm text-[var(--text-secondary)] mb-1">使用するバックエンド</label>
        <select
          id="backend"
          value={config.backend}
          onchange={handleBackendChange}
          class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        >
          <option value="none">なし</option>
          <option value="bouyomichan">棒読みちゃん</option>
          <option value="voicevox">VOICEVOX</option>
        </select>
      </div>

      {#if config.backend === 'bouyomichan'}
        <!-- Bouyomichan Settings -->
        <div class="space-y-3 pt-2 border-t border-[var(--border-default)]">
          <h4 class="text-sm text-[var(--text-secondary)] font-medium">棒読みちゃん設定</h4>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="bouyomi-host" class="block text-xs text-[var(--text-muted)] mb-1">ホスト</label>
              <input
                id="bouyomi-host"
                type="text"
                bind:value={config.bouyomichan_host}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label for="bouyomi-port" class="block text-xs text-[var(--text-muted)] mb-1">ポート</label>
              <input
                id="bouyomi-port"
                type="number"
                bind:value={config.bouyomichan_port}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
          </div>
          <div class="grid grid-cols-4 gap-3">
            <div>
              <label for="bouyomi-voice" class="block text-xs text-[var(--text-muted)] mb-1">声質</label>
              <input
                id="bouyomi-voice"
                type="number"
                bind:value={config.bouyomichan_voice}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label for="bouyomi-volume" class="block text-xs text-[var(--text-muted)] mb-1">音量</label>
              <input
                id="bouyomi-volume"
                type="number"
                bind:value={config.bouyomichan_volume}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label for="bouyomi-speed" class="block text-xs text-[var(--text-muted)] mb-1">速度</label>
              <input
                id="bouyomi-speed"
                type="number"
                bind:value={config.bouyomichan_speed}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label for="bouyomi-tone" class="block text-xs text-[var(--text-muted)] mb-1">トーン</label>
              <input
                id="bouyomi-tone"
                type="number"
                bind:value={config.bouyomichan_tone}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
          </div>
          <p class="text-xs text-[var(--text-muted)]">-1を指定すると棒読みちゃん側の設定が使用されます</p>

          <!-- Auto-launch settings -->
          <div class="pt-3 mt-3 border-t border-[var(--border-default)] space-y-3">
            <h5 class="text-xs text-[var(--text-secondary)] font-medium">自動起動設定</h5>

            <div class="flex items-center justify-between">
              <span class="text-sm text-[var(--text-primary)]">アプリ起動時に自動起動</span>
              <button
                onclick={() => toggleConfig('bouyomichan_auto_launch')}
                data-testid="bouyomichan-auto-launch-toggle"
                aria-pressed={config.bouyomichan_auto_launch}
                class="{config.bouyomichan_auto_launch ? 'bg-[var(--success)]' : 'bg-[var(--bg-surface-3)]'} relative inline-flex h-5 w-9 items-center rounded-full transition-colors"
              >
                <span class="{config.bouyomichan_auto_launch ? 'translate-x-5' : 'translate-x-1'} inline-block h-3 w-3 transform rounded-full bg-white transition-transform shadow"></span>
              </button>
            </div>

            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">実行ファイルパス</label>
              <div class="flex gap-2">
                <input
                  type="text"
                  bind:value={config.bouyomichan_exe_path}
                  oninput={handleConfigChange}
                  placeholder="自動検出または参照で指定"
                  class="flex-1 px-3 py-2 text-sm rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                />
                <button
                  onclick={() => discoverExe('bouyomichan')}
                  class="px-3 py-2 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg border border-[var(--border-default)] hover:bg-[var(--bg-base)] transition-colors"
                  title="自動検出"
                >
                  検出
                </button>
                <button
                  onclick={() => browseExe('bouyomichan')}
                  class="px-3 py-2 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg border border-[var(--border-default)] hover:bg-[var(--bg-base)] transition-colors"
                  title="ファイル参照"
                >
                  参照
                </button>
              </div>
            </div>

            <div class="flex items-center justify-between">
              <span class="text-sm text-[var(--text-primary)]">アプリ終了時に自動停止</span>
              <button
                onclick={() => toggleConfig('bouyomichan_auto_close')}
                data-testid="bouyomichan-auto-close-toggle"
                aria-pressed={config.bouyomichan_auto_close}
                class="{config.bouyomichan_auto_close ? 'bg-[var(--success)]' : 'bg-[var(--bg-surface-3)]'} relative inline-flex h-5 w-9 items-center rounded-full transition-colors"
              >
                <span class="{config.bouyomichan_auto_close ? 'translate-x-5' : 'translate-x-1'} inline-block h-3 w-3 transform rounded-full bg-white transition-transform shadow"></span>
              </button>
            </div>

            <div class="flex items-center gap-3">
              <button
                onclick={() => toggleLaunch('bouyomichan')}
                disabled={isLaunching.bouyomichan}
                data-testid="bouyomichan-launch-button"
                class="px-3 py-2 text-sm rounded-lg border transition-colors disabled:opacity-50 {ttsStore.launchStatus.bouyomichan_launched ? 'bg-[var(--error-subtle)] text-[var(--error)] border-[var(--border-default)] hover:opacity-80' : 'bg-[var(--success-subtle)] text-[var(--success)] border-[var(--border-default)] hover:opacity-80'}"
              >
                {#if isLaunching.bouyomichan}
                  処理中...
                {:else if ttsStore.launchStatus.bouyomichan_launched}
                  停止
                {:else}
                  起動
                {/if}
              </button>
              <span class="text-xs {ttsStore.launchStatus.bouyomichan_launched ? 'text-[var(--success)]' : 'text-[var(--text-muted)]'}">
                {ttsStore.launchStatus.bouyomichan_launched ? '起動中' : '停止中'}
              </span>
            </div>
          </div>
        </div>
      {/if}

      {#if config.backend === 'voicevox'}
        <!-- VOICEVOX Settings -->
        <div class="space-y-3 pt-2 border-t border-[var(--border-default)]">
          <h4 class="text-sm text-[var(--text-secondary)] font-medium">VOICEVOX設定</h4>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="voicevox-host" class="block text-xs text-[var(--text-muted)] mb-1">ホスト</label>
              <input
                id="voicevox-host"
                type="text"
                bind:value={config.voicevox_host}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
            <div>
              <label for="voicevox-port" class="block text-xs text-[var(--text-muted)] mb-1">ポート</label>
              <input
                id="voicevox-port"
                type="number"
                bind:value={config.voicevox_port}
                onchange={handleConfigChange}
                class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
              />
            </div>
          </div>
          <div>
            <label for="voicevox-speaker" class="block text-xs text-[var(--text-muted)] mb-1">話者ID</label>
            <input
              id="voicevox-speaker"
              type="number"
              bind:value={config.voicevox_speaker_id}
              onchange={handleConfigChange}
              class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
            />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="voicevox-volume" class="block text-xs text-[var(--text-muted)] mb-1">音量 ({config.voicevox_volume_scale.toFixed(1)})</label>
              <input
                id="voicevox-volume"
                type="range"
                min="0"
                max="2"
                step="0.1"
                bind:value={config.voicevox_volume_scale}
                oninput={handleConfigChange}
                class="w-full accent-[var(--accent)]"
              />
            </div>
            <div>
              <label for="voicevox-speed" class="block text-xs text-[var(--text-muted)] mb-1">速度 ({config.voicevox_speed_scale.toFixed(1)})</label>
              <input
                id="voicevox-speed"
                type="range"
                min="0.5"
                max="2"
                step="0.1"
                bind:value={config.voicevox_speed_scale}
                oninput={handleConfigChange}
                class="w-full accent-[var(--accent)]"
              />
            </div>
            <div>
              <label for="voicevox-pitch" class="block text-xs text-[var(--text-muted)] mb-1">ピッチ ({config.voicevox_pitch_scale.toFixed(2)})</label>
              <input
                id="voicevox-pitch"
                type="range"
                min="-0.15"
                max="0.15"
                step="0.01"
                bind:value={config.voicevox_pitch_scale}
                oninput={handleConfigChange}
                class="w-full accent-[var(--accent)]"
              />
            </div>
            <div>
              <label for="voicevox-intonation" class="block text-xs text-[var(--text-muted)] mb-1">抑揚 ({config.voicevox_intonation_scale.toFixed(1)})</label>
              <input
                id="voicevox-intonation"
                type="range"
                min="0"
                max="2"
                step="0.1"
                bind:value={config.voicevox_intonation_scale}
                oninput={handleConfigChange}
                class="w-full accent-[var(--accent)]"
              />
            </div>
          </div>

          <!-- Auto-launch settings -->
          <div class="pt-3 mt-3 border-t border-[var(--border-default)] space-y-3">
            <h5 class="text-xs text-[var(--text-secondary)] font-medium">自動起動設定</h5>

            <div class="flex items-center justify-between">
              <span class="text-sm text-[var(--text-primary)]">アプリ起動時に自動起動</span>
              <button
                onclick={() => toggleConfig('voicevox_auto_launch')}
                data-testid="voicevox-auto-launch-toggle"
                aria-pressed={config.voicevox_auto_launch}
                class="{config.voicevox_auto_launch ? 'bg-[var(--success)]' : 'bg-[var(--bg-surface-3)]'} relative inline-flex h-5 w-9 items-center rounded-full transition-colors"
              >
                <span class="{config.voicevox_auto_launch ? 'translate-x-5' : 'translate-x-1'} inline-block h-3 w-3 transform rounded-full bg-white transition-transform shadow"></span>
              </button>
            </div>

            <div>
              <label class="block text-xs text-[var(--text-muted)] mb-1">実行ファイルパス</label>
              <div class="flex gap-2">
                <input
                  type="text"
                  bind:value={config.voicevox_exe_path}
                  oninput={handleConfigChange}
                  placeholder="自動検出または参照で指定"
                  class="flex-1 px-3 py-2 text-sm rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                />
                <button
                  onclick={() => discoverExe('voicevox')}
                  class="px-3 py-2 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg border border-[var(--border-default)] hover:bg-[var(--bg-base)] transition-colors"
                  title="自動検出"
                >
                  検出
                </button>
                <button
                  onclick={() => browseExe('voicevox')}
                  class="px-3 py-2 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg border border-[var(--border-default)] hover:bg-[var(--bg-base)] transition-colors"
                  title="ファイル参照"
                >
                  参照
                </button>
              </div>
            </div>

            <div class="flex items-center justify-between">
              <span class="text-sm text-[var(--text-primary)]">アプリ終了時に自動停止</span>
              <button
                onclick={() => toggleConfig('voicevox_auto_close')}
                data-testid="voicevox-auto-close-toggle"
                aria-pressed={config.voicevox_auto_close}
                class="{config.voicevox_auto_close ? 'bg-[var(--success)]' : 'bg-[var(--bg-surface-3)]'} relative inline-flex h-5 w-9 items-center rounded-full transition-colors"
              >
                <span class="{config.voicevox_auto_close ? 'translate-x-5' : 'translate-x-1'} inline-block h-3 w-3 transform rounded-full bg-white transition-transform shadow"></span>
              </button>
            </div>

            <div class="flex items-center gap-3">
              <button
                onclick={() => toggleLaunch('voicevox')}
                disabled={isLaunching.voicevox}
                data-testid="voicevox-launch-button"
                class="px-3 py-2 text-sm rounded-lg border transition-colors disabled:opacity-50 {ttsStore.launchStatus.voicevox_launched ? 'bg-[var(--error-subtle)] text-[var(--error)] border-[var(--border-default)] hover:opacity-80' : 'bg-[var(--success-subtle)] text-[var(--success)] border-[var(--border-default)] hover:opacity-80'}"
              >
                {#if isLaunching.voicevox}
                  処理中...
                {:else if ttsStore.launchStatus.voicevox_launched}
                  停止
                {:else}
                  起動
                {/if}
              </button>
              <span class="text-xs {ttsStore.launchStatus.voicevox_launched ? 'text-[var(--success)]' : 'text-[var(--text-muted)]'}">
                {ttsStore.launchStatus.voicevox_launched ? '起動中' : '停止中'}
              </span>
            </div>
          </div>
        </div>
      {/if}

      <!-- Connection Test -->
      {#if config.backend !== 'none'}
        <div class="flex items-center gap-4">
          <button
            onclick={testConnection}
            disabled={ttsStore.testingBackend !== null}
            class="px-4 py-2 text-[var(--text-inverse)] rounded-lg transition-colors disabled:opacity-50"
            style="background: var(--accent);"
          >
            {ttsStore.testingBackend ? '接続テスト中...' : '接続テスト'}
          </button>
          {#if ttsStore.connectionTestResult !== null}
            {#if ttsStore.connectionTestResult}
              <span class="text-[var(--success)]">接続成功</span>
            {:else}
              <span class="text-[var(--error)]">接続失敗</span>
            {/if}
          {/if}
        </div>
      {/if}
    </div>

    <!-- Reading Options -->
    <div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] space-y-4">
      <h3 class="text-[var(--text-primary)] font-medium">読み上げオプション</h3>

      <div class="space-y-2">
        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={config.read_author_name}
            onchange={handleConfigChange}
            class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
          />
          <span class="text-[var(--text-primary)] text-sm">投稿者名を読み上げる</span>
        </label>

        {#if config.read_author_name}
          <div class="ml-6 space-y-2">
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                bind:checked={config.add_honorific}
                onchange={handleConfigChange}
                class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
              />
              <span class="text-[var(--text-primary)] text-sm">「さん」を付ける</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                bind:checked={config.strip_at_prefix}
                onchange={handleConfigChange}
                class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
              />
              <span class="text-[var(--text-primary)] text-sm">@プレフィックスを除去</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                bind:checked={config.strip_handle_suffix}
                onchange={handleConfigChange}
                class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
              />
              <span class="text-[var(--text-primary)] text-sm">ハンドルサフィックスを除去</span>
            </label>
          </div>
        {/if}

        <label class="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            bind:checked={config.read_superchat_amount}
            onchange={handleConfigChange}
            class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
          />
          <span class="text-[var(--text-primary)] text-sm">スーパーチャット金額を読み上げる</span>
        </label>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <div>
          <label for="max-length" class="block text-xs text-[var(--text-muted)] mb-1">最大文字数</label>
          <input
            id="max-length"
            type="number"
            min="50"
            max="500"
            bind:value={config.max_text_length}
            onchange={handleConfigChange}
            class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
        <div>
          <label for="queue-limit" class="block text-xs text-[var(--text-muted)] mb-1">キュー上限</label>
          <input
            id="queue-limit"
            type="number"
            min="10"
            max="200"
            bind:value={config.queue_size_limit}
            onchange={handleConfigChange}
            class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
          />
        </div>
      </div>
    </div>

    <!-- Test Section -->
    <div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] space-y-4">
      <h3 class="text-[var(--text-primary)] font-medium">読み上げテスト</h3>

      <div class="flex gap-2">
        <input
          type="text"
          bind:value={testText}
          placeholder="テスト文を入力"
          class="flex-1 px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
        />
        <button
          onclick={testSpeak}
          disabled={!testText || config.backend === 'none' || isSpeaking}
          class="px-4 py-2 text-[var(--text-inverse)] rounded-lg transition-colors disabled:opacity-50 min-w-[100px]"
          style="background: var(--accent);"
        >
          {#if isSpeaking}
            <span class="flex items-center justify-center gap-2">
              <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
            </span>
          {:else}
            読み上げ
          {/if}
        </button>
      </div>
    </div>

    <!-- Queue Control -->
    <div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)]">
      <div class="flex items-center justify-between">
        <div>
          <h3 class="text-[var(--text-primary)] font-medium">キュー管理</h3>
          <p class="text-sm text-[var(--text-secondary)]">現在 {ttsStore.status.queue_size} 件のメッセージが待機中</p>
        </div>
        <button
          onclick={() => ttsStore.clearQueue()}
          disabled={ttsStore.status.queue_size === 0}
          class="px-4 py-2 bg-[var(--error-subtle)] text-[var(--error)] rounded-lg border border-[var(--border-default)] hover:opacity-80 transition-colors disabled:opacity-50"
        >
          キューをクリア
        </button>
      </div>
    </div>

    <!-- Error Display -->
    {#if ttsStore.error}
      <div class="p-3 bg-[var(--error-subtle)] rounded-lg border border-[var(--border-default)]">
        <p class="text-[var(--error)] text-sm">{ttsStore.error}</p>
      </div>
    {/if}
  {/if}
</div>
