<script lang="ts">
	import { tabsStore } from '$lib/stores/tabs.svelte';

	// Derived state from tabs store
	let tabsList = $derived(tabsStore.tabs);
	let activeId = $derived(tabsStore.activeTabId);

	function handleTabClick(tabId: string) {
		tabsStore.switchTab(tabId);
	}

	function handleCreateTab() {
		tabsStore.createTab();
	}

	function handleCloseTab(event: MouseEvent, tabId: string) {
		event.stopPropagation();
		tabsStore.closeTab(tabId);
	}
</script>

<aside class="tab-sidebar">
	<div class="tab-header">
		<span class="tab-header-label">Tabs</span>
		<button
			class="btn-new-tab"
			onclick={handleCreateTab}
			title="New tab"
			aria-label="Create new tab"
		>
			+
		</button>
	</div>

	<div class="tab-list" role="tablist" aria-label="Terminal tabs">
		{#each tabsList as tab (tab.tabId)}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<div
				class="tab-item"
				class:active={tab.tabId === activeId}
				role="tab"
				tabindex="0"
				aria-selected={tab.tabId === activeId}
				onclick={() => handleTabClick(tab.tabId)}
				onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleTabClick(tab.tabId); }}
				title={tab.title}
			>
				<span class="tab-title">{tab.title || 'Terminal'}</span>
				<button
					class="btn-close-tab"
					onclick={(e) => handleCloseTab(e, tab.tabId)}
					title="Close tab"
					aria-label="Close tab {tab.title}"
				>
					&times;
				</button>
			</div>
		{/each}
	</div>

	{#if tabsList.length === 0}
		<div class="tab-empty">No tabs</div>
	{/if}
</aside>

<style>
	.tab-sidebar {
		width: 200px;
		min-width: 200px;
		background: var(--bg-secondary, #1a1a1a);
		border-right: 1px solid var(--border, #333);
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.tab-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 12px;
		border-bottom: 1px solid var(--border, #333);
		flex-shrink: 0;
	}

	.tab-header-label {
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-secondary, #888);
	}

	.btn-new-tab {
		width: 22px;
		height: 22px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		color: var(--text-secondary, #888);
		border: 1px solid var(--border, #444);
		border-radius: 4px;
		cursor: pointer;
		font-size: 14px;
		line-height: 1;
		transition: all 0.15s;
		padding: 0;
	}

	.btn-new-tab:hover {
		background: var(--bg-hover, #333);
		color: var(--text-primary, #d4d4d4);
		border-color: var(--text-secondary, #666);
	}

	.tab-list {
		flex: 1;
		overflow-y: auto;
		padding: 4px 0;
	}

	.tab-item {
		display: flex;
		align-items: center;
		width: 100%;
		padding: 6px 12px;
		background: transparent;
		border: none;
		color: var(--text-secondary, #888);
		cursor: pointer;
		font-size: 13px;
		text-align: left;
		transition: all 0.15s;
		gap: 4px;
	}

	.tab-item:hover {
		background: var(--bg-hover, #2a2a2a);
		color: var(--text-primary, #d4d4d4);
	}

	.tab-item.active {
		background: var(--bg-active, #333);
		color: var(--text-primary, #d4d4d4);
		border-left: 2px solid var(--accent, #22c55e);
		padding-left: 10px;
	}

	.tab-title {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.btn-close-tab {
		width: 18px;
		height: 18px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		color: var(--text-secondary, #666);
		border: none;
		border-radius: 3px;
		cursor: pointer;
		font-size: 14px;
		line-height: 1;
		padding: 0;
		opacity: 0;
		transition: all 0.15s;
		flex-shrink: 0;
	}

	.tab-item:hover .btn-close-tab {
		opacity: 1;
	}

	.btn-close-tab:hover {
		background: rgba(220, 38, 38, 0.3);
		color: #f87171;
	}

	.tab-empty {
		padding: 16px 12px;
		color: var(--text-secondary, #666);
		font-size: 12px;
		text-align: center;
		font-style: italic;
	}

	/* Hidden on mobile */
	@media (max-width: 767px) {
		.tab-sidebar {
			display: none;
		}
	}
</style>
