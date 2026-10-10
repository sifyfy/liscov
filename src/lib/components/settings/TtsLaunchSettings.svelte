<script lang="ts">
  // バックエンドの自動起動設定（04_tts.md「TtsSettings.svelte」）。棒読みちゃん・VOICEVOX で同じ部品を使う
  import { ttsStore } from '$lib/stores';
  import Switch from '$lib/components/ui/Switch.svelte';

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
  <div class="flex items-center justify-between gap-3">
    <span class="text-body">{label}</span>
    <Switch {pressed} {label} {testId} {onclick} />
  </div>
{/snippet}

<div class="flex flex-col gap-3 border-t border-outline-variant pt-3">
  <h5 class="text-label text-fg-2">自動起動設定</h5>

  {@render toggle('アプリ起動時に自動起動', autoLaunch, `${backend}-auto-launch-toggle`, onToggleAutoLaunch)}

  <div class="flex flex-col gap-1">
    <label for="{backend}-exe-path" class="text-caption text-fg-2">実行ファイルパス</label>
    <div class="flex gap-2">
      <input
        id="{backend}-exe-path"
        type="text"
        bind:value={exePath}
        oninput={onExePathChange}
        placeholder="自動検出または参照で指定"
        class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
      />
      <button
        type="button"
        onclick={discoverExe}
        title="自動検出"
        class="shrink-0 rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8"
      >
        検出
      </button>
      <button
        type="button"
        onclick={browseExe}
        title="ファイル参照"
        class="shrink-0 rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8"
      >
        参照
      </button>
    </div>
  </div>

  {@render toggle('アプリ終了時に自動停止', autoClose, `${backend}-auto-close-toggle`, onToggleAutoClose)}

  <div class="flex items-center gap-3">
    <button
      type="button"
      onclick={toggleLaunch}
      disabled={isLaunching}
      data-testid="{backend}-launch-button"
      class="rounded-control border border-outline px-3 py-1 text-label hover:bg-fg-1/8 disabled:opacity-50 {isLaunched
        ? 'text-error'
        : 'text-fg-1'}"
    >
      {#if isLaunching}
        処理中...
      {:else if isLaunched}
        停止
      {:else}
        起動
      {/if}
    </button>
    <span class="flex items-center gap-1.5 text-caption text-fg-2">
      <span class="size-1.5 rounded-full {isLaunched ? 'bg-success' : 'bg-fg-3'}" aria-hidden="true"></span>
      {isLaunched ? '起動中' : '停止中'}
    </span>
  </div>
</div>
