<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { chatStore, configStore, authStore, websocketStore } from '$lib/stores';
  import { AppShell } from '$lib/components/layout';

  onMount(async () => {
    await configStore.load();
    chatStore.initDisplaySettings();
    await authStore.refreshStatus();
    if (authStore.isAuthenticated) {
      authStore.checkSessionValidity();
    }
    await chatStore.setupEventListeners();
    await websocketStore.init();
  });

  onDestroy(() => {
    chatStore.cleanup();
  });
</script>

<AppShell />
