<script lang="ts">
  // バックエンドの自動起動設定（04_tts.md「TtsSettings.svelte」）。棒読みちゃん・VOICEVOX で同じ部品を使う
  import { ttsStore } from '$lib/stores';

  type LaunchBackend = 'bouyomichan' | 'voicevox';

  interface Props {
    backend: LaunchBackend;
    autoLaunch: boolean;
    autoClose: boolean;
    exePath: string | null;
    onToggleAutoLaunch: () => void;
    onToggleAutoClose: () => void;
    /** 実行ファイルパスが変わった（入力・検出・参照）。保存は親が行う */
    onExePathChange: () => void;
  }

  let {
    backend,
    autoLaunch,
    autoClose,
    exePath = $bindable(),
    onToggleAutoLaunch,
    onToggleAutoClose,
    onExePathChange
  }: Props = $props();

  let isLaunching = $state(false);
  let isLaunched = $derived(ttsStore.launchStatus[`${backend}_launched`]);

  function setExePath(path: string | null) {
    if (!path) return;
    exePath = path;
    onExePathChange();
  }

  async function discoverExe() {
    setExePath(await ttsStore.discoverExe(backend));
  }

  async function browseExe() {
    setExePath(await ttsStore.selectExe());
  }

  async function toggleLaunch() {
    isLaunching = true;
    try {
      if (isLaunched) {
        await ttsStore.killBackend(backend);
      } else {
        await ttsStore.launchBackend(backend, exePath ?? undefined);
      }
    } finally {
      isLaunching = false;
    }
  }
</script>

{#snippet toggle(label: string, pressed: boolean, testId: string, onclick: () => void)}
  <div class="flex items-center justify-between">
    <span class="text-sm text-[var(--text-primary)]">{label}</span>
    <button
      {onclick}
      data-testid={testId}
      aria-label={label}
      aria-pressed={pressed}
      class="{pressed ? 'bg-[var(--success)]' : 'bg-[var(--bg-surface-3)]'} relative inline-flex h-5 w-9 items-center rounded-full transition-colors"
    >
      <span class="{pressed ? 'translate-x-5' : 'translate-x-1'} inline-block h-3 w-3 transform rounded-full bg-white transition-transform shadow"></span>
    </button>
  </div>
{/snippet}

<div class="pt-3 mt-3 border-t border-[var(--border-default)] space-y-3">
  <h5 class="text-xs text-[var(--text-secondary)] font-medium">自動起動設定</h5>

  {@render toggle('アプリ起動時に自動起動', autoLaunch, `${backend}-auto-launch-toggle`, onToggleAutoLaunch)}

  <div>
    <label for="{backend}-exe-path" class="block text-xs text-[var(--text-muted)] mb-1">実行ファイルパス</label>
    <div class="flex gap-2">
      <input
        id="{backend}-exe-path"
        type="text"
        bind:value={exePath}
        oninput={onExePathChange}
        placeholder="自動検出または参照で指定"
        class="flex-1 px-3 py-2 text-sm rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
      />
      <button
        onclick={discoverExe}
        class="px-3 py-2 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg border border-[var(--border-default)] hover:bg-[var(--bg-base)] transition-colors"
        title="自動検出"
      >
        検出
      </button>
      <button
        onclick={browseExe}
        class="px-3 py-2 text-xs bg-[var(--bg-surface-3)] text-[var(--text-secondary)] rounded-lg border border-[var(--border-default)] hover:bg-[var(--bg-base)] transition-colors"
        title="ファイル参照"
      >
        参照
      </button>
    </div>
  </div>

  {@render toggle('アプリ終了時に自動停止', autoClose, `${backend}-auto-close-toggle`, onToggleAutoClose)}

  <div class="flex items-center gap-3">
    <button
      onclick={toggleLaunch}
      disabled={isLaunching}
      data-testid="{backend}-launch-button"
      class="px-3 py-2 text-sm rounded-lg border transition-colors disabled:opacity-50 {isLaunched ? 'bg-[var(--error-subtle)] text-[var(--error)] border-[var(--border-default)] hover:opacity-80' : 'bg-[var(--success-subtle)] text-[var(--success)] border-[var(--border-default)] hover:opacity-80'}"
    >
      {#if isLaunching}
        処理中...
      {:else if isLaunched}
        停止
      {:else}
        起動
      {/if}
    </button>
    <span class="text-xs {isLaunched ? 'text-[var(--success)]' : 'text-[var(--text-muted)]'}">
      {isLaunched ? '起動中' : '停止中'}
    </span>
  </div>
</div>
