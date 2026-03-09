<script lang="ts">
  import { RevenueDashboard, ExportPanel } from '$lib/components/analytics';
</script>

<!-- アナリティクスタブ: 収益ダッシュボードとエクスポートパネル -->
<div class="flex-1 p-6 overflow-y-auto bg-[var(--bg-base)]">
  <div class="max-w-6xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-6">
    <div class="lg:col-span-2">
      <RevenueDashboard />
    </div>
    <div>
      <ExportPanel />
    </div>
  </div>
</div>
