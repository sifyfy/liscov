<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import type { SaveConfig } from '$lib/types';

  let config = $state<SaveConfig>({
    enabled: false,
    file_path: 'raw_responses.ndjson',
    max_file_size_mb: 100,
    enable_rotation: true,
    max_backup_files: 5
  });

  let resolvedPath = $state('');
  let isLoading = $state(true);
  let saveMessage = $state('');

  // Load config on mount
  $effect(() => {
    loadConfig();
  });

  // Update resolved path when file_path changes
  $effect(() => {
    if (config.file_path) {
      updateResolvedPath(config.file_path);
    }
  });

  async function loadConfig() {
    try {
      const loadedConfig = await invoke<SaveConfig>('raw_response_get_config');
      config = loadedConfig;
      isLoading = false;
    } catch (error) {
      console.error('Failed to load save config:', error);
      isLoading = false;
    }
  }

  async function updateResolvedPath(filePath: string) {
    try {
      resolvedPath = await invoke<string>('raw_response_resolve_path', { filePath });
    } catch (error) {
      console.error('Failed to resolve path:', error);
      resolvedPath = filePath;
    }
  }

  async function saveConfig() {
    try {
      await invoke('raw_response_update_config', { config });
      saveMessage = '設定を保存しました';
      setTimeout(() => saveMessage = '', 3000);
    } catch (error) {
      console.error('Failed to save config:', error);
      saveMessage = `保存に失敗: ${error}`;
      // 拒否された設定はバックエンドに反映されていないので、表示を実際の設定に戻す
      await loadConfig();
    }
  }

  async function browseFile() {
    try {
      const selected = await save({
        title: '生レスポンス保存先を指定',
        filters: [
          { name: 'NDJSON ファイル', extensions: ['ndjson'] },
          { name: 'JSON ファイル', extensions: ['json'] },
          { name: 'すべてのファイル', extensions: ['*'] }
        ],
        defaultPath: config.file_path || 'raw_responses.ndjson'
      });

      if (selected) {
        config.file_path = selected;
        await saveConfig();
      }
    } catch (error) {
      console.error('Failed to open file dialog:', error);
    }
  }

  function handleEnabledChange(event: Event) {
    const target = event.target as HTMLInputElement;
    config.enabled = target.checked;
    saveConfig();
  }

  function handleRotationChange(event: Event) {
    const target = event.target as HTMLInputElement;
    config.enable_rotation = target.checked;
    saveConfig();
  }

  function handleFilePathChange(event: Event) {
    const target = event.target as HTMLInputElement;
    config.file_path = target.value;
  }

  function handleFilePathBlur() {
    saveConfig();
  }

  function handleMaxSizeChange(event: Event) {
    const target = event.target as HTMLInputElement;
    const value = parseInt(target.value, 10);
    if (!isNaN(value) && value > 0) {
      config.max_file_size_mb = value;
      saveConfig();
    }
  }
</script>

<div class="flex flex-col gap-4 p-4 text-fg-1">
  <h2 class="text-title">生レスポンス保存設定</h2>

  {#if isLoading}
    <p class="text-body text-fg-2">読み込み中...</p>
  {:else}
    <div class="flex flex-col gap-1">
      <label class="flex items-center gap-2 text-label">
        <input type="checkbox" checked={config.enabled} onchange={handleEnabledChange} class="accent-accent" />
        生レスポンス保存を有効化
      </label>
      <p class="pl-6 text-body text-fg-2">YouTube InnerTube APIからの生レスポンスをndjson形式で保存します</p>
    </div>

    {#if config.enabled}
      <section class="flex flex-col gap-2 border-t border-outline-variant pt-4">
        <label for="raw-file-path" class="text-label">保存ファイルパス</label>
        <div class="flex gap-2">
          <input
            id="raw-file-path"
            type="text"
            value={config.file_path}
            oninput={handleFilePathChange}
            onblur={handleFilePathBlur}
            placeholder="例: C:\Users\Username\Documents\raw_responses.ndjson"
            class="min-w-0 flex-1 rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
          />
          <button
            type="button"
            onclick={browseFile}
            class="shrink-0 rounded-control border border-outline px-3 py-1 text-label text-fg-2 hover:bg-fg-1/8"
          >
            参照
          </button>
        </div>
        <p class="flex flex-wrap items-baseline gap-x-2 text-caption text-fg-2">
          実際の保存先:
          <code class="font-code break-all text-fg-1">{resolvedPath}</code>
        </p>
      </section>

      <section class="flex flex-col gap-2 border-t border-outline-variant pt-4">
        <label for="raw-max-size" class="text-label">最大ファイルサイズ (MB)</label>
        <input
          id="raw-max-size"
          type="number"
          value={config.max_file_size_mb}
          onchange={handleMaxSizeChange}
          min="1"
          max="1000"
          class="w-32 rounded-control border border-outline bg-surface px-2 py-1.5 text-body text-fg-1 placeholder:text-fg-3 focus:border-accent"
        />
      </section>

      <section class="flex flex-col gap-2 border-t border-outline-variant pt-4">
        <label class="flex items-center gap-2 text-label">
          <input type="checkbox" checked={config.enable_rotation} onchange={handleRotationChange} class="accent-accent" />
          ファイルローテーションを有効化
        </label>
        <p class="pl-6 text-body text-fg-2">サイズ上限に達すると自動で新しいファイルが作成されます</p>
      </section>

      <section class="flex flex-col gap-2 border-t border-outline-variant pt-4">
        <h3 class="text-label">ヒント</h3>
        <ul class="flex list-disc flex-col gap-1 pl-5 text-body text-fg-2">
          <li>生レスポンスは将来のAPI変更に対応するために保存されます</li>
          <li>ファイルはndjson形式で保存されます</li>
          <li>ローテーション有効時、サイズ上限に達すると自動で新ファイルが作成されます</li>
          <li>最大{config.max_backup_files}個のバックアップファイルが保持されます</li>
        </ul>
      </section>
    {/if}

    {#if saveMessage}
      <p role="status" class="text-body {saveMessage.startsWith('保存に失敗') ? 'text-error' : 'text-success'}">{saveMessage}</p>
    {/if}
  {/if}
</div>
