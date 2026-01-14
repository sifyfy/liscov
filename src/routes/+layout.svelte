<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<div class="min-h-screen bg-[var(--bg-main)]">
	{@render children()}
</div>
