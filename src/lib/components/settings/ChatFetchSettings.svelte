<script lang="ts">
  // チャット取得の設定 (02_chat.md「プッシュ通知 > 設定の画面」)。配信中に通知が原因の問題が起きたときの回避策
  import { configStore } from '$lib/stores';
</script>

<div class="flex flex-col gap-4 p-4 text-fg-1">
  <header class="flex flex-col gap-1">
    <h2 class="text-title">チャット取得</h2>
  </header>

  <label class="flex items-start gap-2">
    <input
      type="checkbox"
      checked={configStore.pushNotification}
      onchange={(e) => configStore.setPushNotification(e.currentTarget.checked)}
      aria-describedby="push-notification-description"
      class="mt-1 accent-accent"
    />
    <span class="flex flex-col gap-1">
      <span class="text-label">プッシュ通知でコメントを早く取る（実験的）</span>
      <span id="push-notification-description" class="text-body text-fg-2">
        YouTube からの通知を受けて、静かなときの最初のコメントを早く取ります。コメントの取得がおかしいときはオフにしてください。オフでも今までどおり取得します。
      </span>
    </span>
  </label>

  {#if configStore.error}
    <p role="alert" class="text-body text-error">{configStore.error}</p>
  {/if}
</div>
