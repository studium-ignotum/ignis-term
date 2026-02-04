<script lang="ts">
  import { connectionStore } from '$lib/stores/connection.svelte';

  // Map state to display info
  const stateDisplay = {
    disconnected: { label: 'Disconnected', color: 'text-gray-500', icon: '○' },
    connecting: { label: 'Connecting...', color: 'text-yellow-500', icon: '◐' },
    authenticating: { label: 'Authenticating...', color: 'text-yellow-500', icon: '◑' },
    connected: { label: 'Connected', color: 'text-green-500', icon: '●' },
    reconnecting: { label: 'Reconnecting...', color: 'text-orange-500', icon: '◐' },
  } as const;

  // Reactive derivation from store state
  let display = $derived(stateDisplay[connectionStore.state]);
</script>

<div class="connection-status">
  <span class="icon {display.color}">{display.icon}</span>
  <span class="label {display.color}">{display.label}</span>
</div>

<style>
  .connection-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    border-radius: 9999px;
    background-color: rgba(31, 41, 55, 0.5);
    font-size: 0.875rem;
  }

  .icon {
    font-size: 0.75rem;
  }

  .label {
    font-weight: 500;
  }

  /* Color classes using CSS variables for theme consistency */
  .text-gray-500 {
    color: var(--text-secondary, #6b7280);
  }

  .text-yellow-500 {
    color: #eab308;
  }

  .text-green-500 {
    color: #22c55e;
  }

  .text-orange-500 {
    color: #f97316;
  }
</style>
