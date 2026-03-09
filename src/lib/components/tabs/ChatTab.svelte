<script lang="ts">
  import { ChatDisplay, FilterPanel, InputSection } from '$lib/components/chat';
</script>

<!-- チャット入力セクション -->
<div class="bg-[var(--bg-surface-1)] border-b" style="border-color: var(--border-subtle);">
  <InputSection />
</div>
<!-- フィルターパネル -->
<FilterPanel />
<!-- チャット表示エリア -->
<div class="flex-1 overflow-hidden">
  <ChatDisplay />
</div>
