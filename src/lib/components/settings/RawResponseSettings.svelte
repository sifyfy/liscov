<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';

  interface SaveConfig {
    enabled: boolean;
    file_path: string;
    max_file_size_mb: number;
    enable_rotation: boolean;
    max_backup_files: number;
  }

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

<div class="p-6">
  <h2 class="text-xl font-bold text-[var(--text-primary)] mb-6" style="font-family: var(--font-heading);">
    生レスポンス保存設定
  </h2>

  {#if isLoading}
    <div class="text-[var(--text-secondary)]">読み込み中...</div>
  {:else}
    <div class="space-y-6">
      <!-- Enable/Disable Toggle -->
      <div class="bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] p-4">
        <label class="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={config.enabled}
            onchange={handleEnabledChange}
            class="w-5 h-5 rounded border-[var(--border-default)] text-[var(--accent)] focus:ring-[var(--accent)]"
          />
          <span class="text-[var(--text-primary)] font-medium">
            生レスポンス保存を有効化
          </span>
        </label>
        <p class="mt-2 text-sm text-[var(--text-muted)] ml-8">
          YouTube InnerTube APIからの生レスポンスをndjson形式で保存します
        </p>
      </div>

      {#if config.enabled}
        <!-- File Path -->
        <div class="bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] p-4">
          <label class="block">
            <span class="text-[var(--text-primary)] font-medium">保存ファイルパス</span>
            <div class="flex gap-2 mt-2">
              <input
                type="text"
                value={config.file_path}
                oninput={handleFilePathChange}
                onblur={handleFilePathBlur}
                placeholder="例: C:\Users\Username\Documents\raw_responses.ndjson"
                class="flex-1 px-3 py-2 border border-[var(--border-default)] rounded-lg text-[var(--text-primary)] bg-[var(--bg-surface-2)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
              />
              <button
                type="button"
                onclick={browseFile}
                class="px-4 py-2 text-[var(--text-inverse)] rounded-lg transition-colors"
                style="background: var(--accent);"
              >
                参照
              </button>
            </div>
          </label>
        </div>

        <!-- Resolved Path Display -->
        <div class="bg-[var(--bg-surface-3)] rounded-lg border border-[var(--border-default)] p-4">
          <div class="flex items-start gap-2">
            <span class="text-[var(--text-muted)] text-sm">実際の保存先:</span>
            <code class="text-sm text-[var(--text-primary)] bg-[var(--bg-surface-2)] px-2 py-1 rounded border border-[var(--border-default)] break-all">
              {resolvedPath}
            </code>
          </div>
        </div>

        <!-- Max File Size -->
        <div class="bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] p-4">
          <label class="block">
            <span class="text-[var(--text-primary)] font-medium">最大ファイルサイズ (MB)</span>
            <input
              type="number"
              value={config.max_file_size_mb}
              onchange={handleMaxSizeChange}
              min="1"
              max="1000"
              class="mt-2 w-32 px-3 py-2 border border-[var(--border-default)] rounded-lg text-[var(--text-primary)] bg-[var(--bg-surface-2)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]/50"
            />
          </label>
        </div>

        <!-- File Rotation -->
        <div class="bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] p-4">
          <label class="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              checked={config.enable_rotation}
              onchange={handleRotationChange}
              class="w-5 h-5 rounded border-[var(--border-default)] text-[var(--accent)] focus:ring-[var(--accent)]"
            />
            <span class="text-[var(--text-primary)] font-medium">
              ファイルローテーションを有効化
            </span>
          </label>
          <p class="mt-2 text-sm text-[var(--text-muted)] ml-8">
            サイズ上限に達すると自動で新しいファイルが作成されます
          </p>
        </div>

        <!-- Info Box -->
        <div class="bg-[var(--info-subtle)] border border-[var(--border-default)] rounded-lg p-4">
          <h4 class="font-medium text-[var(--info)] mb-2">ヒント</h4>
          <ul class="text-sm text-[var(--info)] space-y-1 list-disc list-inside">
            <li>生レスポンスは将来のAPI変更に対応するために保存されます</li>
            <li>ファイルはndjson形式で保存されます</li>
            <li>ローテーション有効時、サイズ上限に達すると自動で新ファイルが作成されます</li>
            <li>最大{config.max_backup_files}個のバックアップファイルが保持されます</li>
          </ul>
        </div>
      {/if}

      <!-- Save Message -->
      {#if saveMessage}
        <div class="text-sm text-[var(--success)] bg-[var(--success-subtle)] border border-[var(--border-default)] rounded-lg px-4 py-2">
          {saveMessage}
        </div>
      {/if}
    </div>
  {/if}
</div>
