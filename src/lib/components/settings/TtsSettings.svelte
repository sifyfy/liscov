<script lang="ts">
  import { ttsStore } from '$lib/stores';
  import type { TtsBackend, TtsConfig } from '$lib/types';
  import { onMount } from 'svelte';
  import TtsLaunchSettings from './TtsLaunchSettings.svelte';
  import Switch from '$lib/components/ui/Switch.svelte';

  // Local state for editing
  let config = $state<TtsConfig | null>(null);
  let testText = $state('テスト読み上げです');
  let isSpeaking = $state(false);

  onMount(() => {
    loadConfig();
    ttsStore.refreshLaunchStatus();
  });

  async function loadConfig() {
    await ttsStore.loadConfig();
    config = { ...ttsStore.config };
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

<div class="flex flex-col gap-4 p-4 text-fg-1">
  <header class="flex flex-wrap items-center justify-between gap-x-3 gap-y-1">
    <h2 class="text-title">TTS設定</h2>
    <span class="flex items-center gap-3 text-caption text-fg-2 tabular-nums">
      キュー: {ttsStore.status.queue_size}件
      {#if ttsStore.status.is_processing}
        <span class="flex items-center gap-1.5">
          <span class="size-1.5 rounded-full bg-success" aria-hidden="true"></span>
          処理中
        </span>
      {/if}
    </span>
  </header>

  {#if ttsStore.isLoading}
    <p class="text-body text-fg-2">読み込み中...</p>
  {:else if config}
    <div class="flex items-center justify-between gap-3">
      <div class="flex flex-col gap-1">
        <h3 class="text-label">TTS読み上げ</h3>
        <p class="text-body text-fg-2">チャットメッセージを音声で読み上げます</p>
      </div>
      <Switch pressed={config.enabled} label={config.enabled ? 'TTSを無効にする' : 'TTSを有効にする'} onclick={toggleEnabled} />
    </div>

    <section class="flex flex-col gap-3 border-t border-outline-variant pt-4">
      <h3 class="text-label">バックエンド設定</h3>
      <label class="flex flex-col gap-1 text-caption text-fg-2">
        使用するバックエンド
        <select id="backend" value={config.backend} onchange={handleBackendChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent">
          <option value="none">なし</option>
          <option value="bouyomichan">棒読みちゃん</option>
          <option value="voicevox">VOICEVOX</option>
        </select>
      </label>

      {#if config.backend === 'bouyomichan'}
        <div class="flex flex-col gap-3 border-t border-outline-variant pt-3">
          <h4 class="text-label text-fg-2">棒読みちゃん設定</h4>
          <div class="grid grid-cols-2 gap-3">
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              ホスト
              <input id="bouyomi-host" type="text" bind:value={config.bouyomichan_host} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              ポート
              <input id="bouyomi-port" type="number" bind:value={config.bouyomichan_port} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
          </div>
          <div class="grid grid-cols-4 gap-3">
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              声質
              <input id="bouyomi-voice" type="number" bind:value={config.bouyomichan_voice} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              音量
              <input id="bouyomi-volume" type="number" bind:value={config.bouyomichan_volume} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              速度
              <input id="bouyomi-speed" type="number" bind:value={config.bouyomichan_speed} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              トーン
              <input id="bouyomi-tone" type="number" bind:value={config.bouyomichan_tone} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
          </div>
          <p class="text-caption text-fg-2">-1を指定すると棒読みちゃん側の設定が使用されます</p>

          <TtsLaunchSettings
            backend="bouyomichan"
            autoLaunch={config.bouyomichan_auto_launch}
            autoClose={config.bouyomichan_auto_close}
            bind:exePath={config.bouyomichan_exe_path}
            onToggleAutoLaunch={() => toggleConfig('bouyomichan_auto_launch')}
            onToggleAutoClose={() => toggleConfig('bouyomichan_auto_close')}
            onExePathChange={handleConfigChange}
          />
        </div>
      {/if}

      {#if config.backend === 'voicevox'}
        <div class="flex flex-col gap-3 border-t border-outline-variant pt-3">
          <h4 class="text-label text-fg-2">VOICEVOX設定</h4>
          <div class="grid grid-cols-2 gap-3">
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              ホスト
              <input id="voicevox-host" type="text" bind:value={config.voicevox_host} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              ポート
              <input id="voicevox-port" type="number" bind:value={config.voicevox_port} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
            </label>
          </div>
          <label class="flex flex-col gap-1 text-caption text-fg-2">
            話者ID
            <input id="voicevox-speaker" type="number" bind:value={config.voicevox_speaker_id} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
          </label>
          <div class="grid grid-cols-2 gap-3">
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              音量 ({config.voicevox_volume_scale.toFixed(1)})
              <input id="voicevox-volume" type="range" min="0" max="2" step="0.1" bind:value={config.voicevox_volume_scale} oninput={handleConfigChange} class="w-full accent-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              速度 ({config.voicevox_speed_scale.toFixed(1)})
              <input id="voicevox-speed" type="range" min="0.5" max="2" step="0.1" bind:value={config.voicevox_speed_scale} oninput={handleConfigChange} class="w-full accent-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              ピッチ ({config.voicevox_pitch_scale.toFixed(2)})
              <input id="voicevox-pitch" type="range" min="-0.15" max="0.15" step="0.01" bind:value={config.voicevox_pitch_scale} oninput={handleConfigChange} class="w-full accent-accent" />
            </label>
            <label class="flex flex-col gap-1 text-caption text-fg-2">
              抑揚 ({config.voicevox_intonation_scale.toFixed(1)})
              <input id="voicevox-intonation" type="range" min="0" max="2" step="0.1" bind:value={config.voicevox_intonation_scale} oninput={handleConfigChange} class="w-full accent-accent" />
            </label>
          </div>

          <TtsLaunchSettings
            backend="voicevox"
            autoLaunch={config.voicevox_auto_launch}
            autoClose={config.voicevox_auto_close}
            bind:exePath={config.voicevox_exe_path}
            onToggleAutoLaunch={() => toggleConfig('voicevox_auto_launch')}
            onToggleAutoClose={() => toggleConfig('voicevox_auto_close')}
            onExePathChange={handleConfigChange}
          />
        </div>
      {/if}

      {#if config.backend !== 'none'}
        <div class="flex items-center gap-3">
          <button
            type="button"
            onclick={testConnection}
            disabled={ttsStore.testingBackend !== null}
            class="rounded-control border border-outline px-3 py-1 text-label text-fg-1 hover:bg-fg-1/8 disabled:opacity-50"
          >
            {ttsStore.testingBackend ? '接続テスト中...' : '接続テスト'}
          </button>
          {#if ttsStore.connectionTestResult !== null}
            {#if ttsStore.connectionTestResult}
              <span class="text-body text-success">接続成功</span>
            {:else}
              <span class="text-body text-error">接続失敗</span>
            {/if}
          {/if}
        </div>
      {/if}
    </section>

    <section class="flex flex-col gap-3 border-t border-outline-variant pt-4">
      <h3 class="text-label">読み上げオプション</h3>
      <div class="flex flex-col gap-2">
        <label class="flex items-center gap-2 text-body">
          <input type="checkbox" bind:checked={config.read_author_name} onchange={handleConfigChange} class="accent-accent" />
          投稿者名を読み上げる
        </label>
        {#if config.read_author_name}
          <div class="flex flex-col gap-2 pl-6">
            <label class="flex items-center gap-2 text-body">
              <input type="checkbox" bind:checked={config.add_honorific} onchange={handleConfigChange} class="accent-accent" />
              「さん」を付ける
            </label>
            <label class="flex items-center gap-2 text-body">
              <input type="checkbox" bind:checked={config.strip_at_prefix} onchange={handleConfigChange} class="accent-accent" />
              @プレフィックスを除去
            </label>
            <label class="flex items-center gap-2 text-body">
              <input type="checkbox" bind:checked={config.strip_handle_suffix} onchange={handleConfigChange} class="accent-accent" />
              ハンドルサフィックスを除去
            </label>
          </div>
        {/if}
        <label class="flex items-center gap-2 text-body">
          <input type="checkbox" bind:checked={config.read_superchat_amount} onchange={handleConfigChange} class="accent-accent" />
          スーパーチャット金額を読み上げる
        </label>
      </div>

      <div class="flex flex-col gap-3 border-t border-outline-variant pt-3">
        <div class="flex items-center justify-between gap-3">
          <div class="flex flex-col">
            <span class="text-body">初回コメントにプレフィックスを付ける</span>
            <span class="text-caption text-fg-2">配信内で初めてコメントした視聴者のメッセージに文言を付加</span>
          </div>
          <Switch
            pressed={config.first_comment_prefix_enabled}
            label="初回コメントにプレフィックスを付ける"
            testId="first-comment-prefix-toggle"
            onclick={() => {
              if (config) {
                config.first_comment_prefix_enabled = !config.first_comment_prefix_enabled;
                handleConfigChange();
              }
            }}
          />
        </div>
        {#if config.first_comment_prefix_enabled}
          <label class="flex flex-col gap-1 text-caption text-fg-2 pl-6">
            プレフィックス文言
            <input
              id="first-comment-prefix"
              type="text"
              bind:value={config.first_comment_prefix}
              oninput={handleConfigChange}
              placeholder="1回目のコメント。"
              data-testid="first-comment-prefix-input"
              class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
            />
          </label>
        {/if}
        <div class="flex items-center justify-between gap-3">
          <div class="flex flex-col">
            <span class="text-body">初回コメントのみ読み上げる</span>
            <span class="text-caption text-fg-2">各視聴者の配信内1回目のコメントだけを読み上げ</span>
          </div>
          <Switch
            pressed={config.first_comment_only}
            label="初回コメントのみ読み上げる"
            testId="first-comment-only-toggle"
            onclick={() => {
              if (config) {
                config.first_comment_only = !config.first_comment_only;
                handleConfigChange();
              }
            }}
          />
        </div>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <label class="flex flex-col gap-1 text-caption text-fg-2">
          最大文字数
          <input id="max-length" type="number" min="50" max="500" bind:value={config.max_text_length} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
        </label>
        <label class="flex flex-col gap-1 text-caption text-fg-2">
          キュー上限
          <input id="queue-limit" type="number" min="10" max="200" bind:value={config.queue_size_limit} onchange={handleConfigChange} class="w-full rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent" />
        </label>
      </div>
    </section>

    <section class="flex flex-col gap-3 border-t border-outline-variant pt-4">
      <h3 class="text-label">読み上げテスト</h3>
      <div class="flex gap-2">
        <input
          type="text"
          bind:value={testText}
          placeholder="テスト文を入力"
          aria-label="テスト文"
          class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
        />
        <button
          type="button"
          onclick={testSpeak}
          disabled={!testText || config.backend === 'none' || isSpeaking}
          class="shrink-0 rounded-control bg-accent px-4 py-1.5 text-label text-surface hover:opacity-90 disabled:opacity-50"
        >
          {isSpeaking ? '読み上げ中...' : '読み上げ'}
        </button>
      </div>
    </section>

    <section class="flex items-center justify-between gap-3 border-t border-outline-variant pt-4">
      <div class="flex flex-col gap-1">
        <h3 class="text-label">キュー管理</h3>
        <p class="text-body text-fg-2">現在 {ttsStore.status.queue_size} 件のメッセージが待機中</p>
      </div>
      <button
        type="button"
        onclick={() => ttsStore.clearQueue()}
        disabled={ttsStore.status.queue_size === 0}
        class="shrink-0 rounded-control border border-outline px-3 py-1 text-label text-error hover:bg-fg-1/8 disabled:opacity-50"
      >
        キューをクリア
      </button>
    </section>

    {#if ttsStore.error}
      <p role="alert" class="text-body text-error">{ttsStore.error}</p>
    {/if}
  {/if}
</div>
