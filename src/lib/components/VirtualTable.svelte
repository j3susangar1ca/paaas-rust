<script lang="ts">
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { appState } from '$lib/store/dataState.svelte';

  let parentEl = $state<HTMLDivElement | null>(null);

  const virtualizer = createVirtualizer({
    get count() { return appState.filteredRows.length; },
    getScrollElement: () => parentEl,
    estimateSize: () => 40,
    overscan: 15,
  });

  let virtualItems = $derived($virtualizer.getVirtualItems());
  let totalSize = $derived($virtualizer.getTotalSize());
</script>

<div class="h-full w-full flex flex-col bg-slate-900 border border-slate-800 rounded-xl overflow-x-auto overflow-y-hidden shadow-2xl">
  <div class="min-w-max flex flex-col h-full flex-1">
    <!-- Table Header -->
    <div class="overflow-hidden border-b border-slate-800 bg-slate-950 text-slate-200" style="scrollbar-gutter: stable;">
      <div
        class="grid font-bold text-xs uppercase tracking-wider"
        style="grid-template-columns: repeat({appState.columns.length}, minmax(180px, 1fr));"
      >
        {#each appState.columns as col}
          <div class="p-3 border-r border-slate-800 last:border-r-0 truncate">
            {col}
          </div>
        {/each}
      </div>
    </div>

    <!-- Table Body (Scroll Container) -->
    <div class="flex-1 overflow-y-auto overflow-x-hidden bg-slate-900/60" bind:this={parentEl} style="scrollbar-gutter: stable;">
      {#if appState.filteredRows.length === 0}
        <div class="flex flex-col items-center justify-center h-full text-slate-500 py-12">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 mb-3 opacity-60 text-slate-400 animate-pulse">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 7.5l-.625 10.632a2.25 2.25 0 01-2.247 2.118H6.622a2.25 2.25 0 01-2.247-2.118L3.75 7.5m8.25 3v6.75m0 0l-3-3m3 3l3-3M3.375 7.5h17.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125z" />
          </svg>
          <span class="text-sm font-medium">No se encontraron registros</span>
        </div>
      {:else}
        <div class="relative w-full" style="height: {totalSize}px">
          {#each virtualItems as virtualRow (virtualRow.index)}
            {@const row = appState.filteredRows[virtualRow.index]}
            <div
              class="absolute top-0 left-0 w-full grid border-b border-slate-800/40 hover:bg-slate-800/30 transition-colors text-slate-300 text-sm align-middle items-center"
              style="height: {virtualRow.size}px; transform: translateY({virtualRow.start}px); grid-template-columns: repeat({appState.columns.length}, minmax(180px, 1fr));"
            >
              {#each appState.columns as col}
                <div class="p-3 border-r border-slate-800/40 last:border-r-0 truncate max-w-full select-text" title={row[col]}>
                  {#if row[col] === null || row[col] === undefined}
                    <span class="text-slate-600 italic">null</span>
                  {:else if typeof row[col] === 'number'}
                    <span class="font-mono text-emerald-400">
                      {row[col].toLocaleString('es-ES', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                    </span>
                  {:else}
                    {row[col]}
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
