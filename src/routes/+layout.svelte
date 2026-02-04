<script lang="ts">
	import '../app.css';
	import ConnectionStatus from '$lib/components/ConnectionStatus.svelte';
	import { connectionStore } from '$lib/stores/connection';

	let { children } = $props();
</script>

{#if connectionStore.state !== 'disconnected'}
	<header class="connection-header">
		<ConnectionStatus />
	</header>
{/if}

{@render children()}

<style>
	.connection-header {
		position: fixed;
		top: 0;
		right: 0;
		padding: 16px;
		z-index: 50;
	}
</style>
