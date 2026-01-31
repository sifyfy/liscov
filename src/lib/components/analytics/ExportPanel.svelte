<script lang="ts">
  import { analyticsStore } from '$lib/stores';
  import type { ExportConfig } from '$lib/types';

  interface Props {
    sessionId?: string;
  }

  let { sessionId }: Props = $props();

  let format = $state<'csv' | 'json'>('json');
  let includeMetadata = $state(true);
  let includeSystemMessages = $state(false);
  let maxRecords = $state<number | undefined>(undefined);
  let isExporting = $state(false);
  let exportError = $state<string | null>(null);
  let exportSuccess = $state(false);

  async function handleExport() {
    isExporting = true;
    exportError = null;
    exportSuccess = false;

    const config: ExportConfig = {
      format,
      include_metadata: includeMetadata,
      include_system_messages: includeSystemMessages,
      max_records: maxRecords
    };

    // Generate filename
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
    const filename = `liscov-export-${timestamp}.${format}`;

    // Use file dialog to get save path
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');

      const filePath = await save({
        defaultPath: filename,
        filters: [
          {
            name: format === 'json' ? 'JSON' : 'CSV',
            extensions: [format]
          }
        ]
      });

      if (!filePath) {
        isExporting = false;
        return;
      }

      if (sessionId) {
        await analyticsStore.exportSession(sessionId, filePath, config);
      } else {
        await analyticsStore.exportCurrent(filePath, config);
      }

      exportSuccess = true;
      setTimeout(() => {
        exportSuccess = false;
      }, 3000);
    } catch (e) {
      exportError = e instanceof Error ? e.message : String(e);
    } finally {
      isExporting = false;
    }
  }
</script>

<div class="p-4 bg-[var(--bg-surface-2)] rounded-lg border border-[var(--border-default)] space-y-4">
  <h3 class="text-lg font-medium text-[var(--text-primary)]">Export Data</h3>

  <!-- Format selection -->
  <div>
    <span class="block text-sm text-[var(--text-secondary)] mb-2">Format</span>
    <div class="flex gap-4">
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="radio"
          name="format"
          value="json"
          bind:group={format}
          class="text-[var(--accent)] focus:ring-[var(--accent)]"
        />
        <span class="text-[var(--text-primary)]">JSON</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input
          type="radio"
          name="format"
          value="csv"
          bind:group={format}
          class="text-[var(--accent)] focus:ring-[var(--accent)]"
        />
        <span class="text-[var(--text-primary)]">CSV</span>
      </label>
    </div>
  </div>

  <!-- Options -->
  <div class="space-y-2">
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={includeMetadata}
        class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
      />
      <span class="text-[var(--text-primary)] text-sm">Include metadata</span>
    </label>
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        bind:checked={includeSystemMessages}
        class="rounded text-[var(--accent)] focus:ring-[var(--accent)]"
      />
      <span class="text-[var(--text-primary)] text-sm">Include system messages</span>
    </label>
  </div>

  <!-- Max records -->
  <div>
    <label for="max-records" class="block text-sm text-[var(--text-secondary)] mb-1">Max records (optional)</label>
    <input
      id="max-records"
      type="number"
      bind:value={maxRecords}
      min="1"
      placeholder="All records"
      class="w-full px-3 py-2 rounded-lg bg-[var(--bg-surface-3)] text-[var(--text-primary)] placeholder-[var(--text-muted)] border border-[var(--border-default)] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
    />
  </div>

  <!-- Error/Success messages -->
  {#if exportError}
    <div class="p-3 bg-[var(--error-subtle)] rounded-lg border border-[var(--border-default)]">
      <p class="text-[var(--error)] text-sm">{exportError}</p>
    </div>
  {/if}

  {#if exportSuccess}
    <div class="p-3 bg-[var(--success-subtle)] rounded-lg border border-[var(--border-default)]">
      <p class="text-[var(--success)] text-sm">Export completed successfully!</p>
    </div>
  {/if}

  <!-- Export button -->
  <button
    onclick={handleExport}
    disabled={isExporting}
    class="w-full px-4 py-2 text-[var(--text-inverse)] font-semibold rounded-lg transition-colors disabled:opacity-50"
    style="background: var(--accent);"
  >
    {isExporting ? 'Exporting...' : sessionId ? 'Export Session' : 'Export Current Messages'}
  </button>
</div>
