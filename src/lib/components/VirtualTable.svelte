<script lang="ts">
  import { createVirtualizer } from '@tanstack/svelte-virtual';
  import { appState } from '$lib/store/dataState.svelte';

  let parentEl = $state<HTMLDivElement | null>(null);

  const virtualizer = createVirtualizer({
    get count() { return appState.filteredRows.length; },
    getScrollElement: () => parentEl,
    estimateSize: () => 40,
    overscan: 15,
    paddingStart: 40, // Offsets the virtual items by the height of the sticky header
  });

  $effect(() => {
    $virtualizer.setOptions({
      count: appState.filteredRows.length,
      getScrollElement: () => parentEl,
    });
  });

  let virtualItems = $derived($virtualizer.getVirtualItems());
  let totalSize = $derived($virtualizer.getTotalSize());
</script>

<div class="h-full w-full flex flex-col bg-white border border-zinc-200 rounded-xl overflow-hidden shadow-sm">
  <!-- Parent scroll element that scrolls both vertically and horizontally -->
  <div 
    class="flex-1 overflow-auto bg-white" 
    bind:this={parentEl}
  >
    <div class="min-w-max relative flex flex-col min-h-full" style="height: {appState.filteredRows.length > 0 ? totalSize : '100%'}px;">
      <!-- Table Header (Sticky) -->
      <div 
        class="sticky top-0 z-10 font-semibold text-[10px] uppercase tracking-wider bg-zinc-50/90 backdrop-blur-md text-zinc-500 border-b border-zinc-200/80 select-none"
        style="height: 40px;"
      >
        <div
          class="grid h-full items-center"
          style="grid-template-columns: repeat({appState.columns.length}, minmax(180px, 1fr));"
        >
          {#each appState.columns as col}
            <div class="p-3 border-r border-zinc-200/50 last:border-r-0 truncate">
              {col}
            </div>
          {/each}
        </div>
      </div>

      <!-- Table Body -->
      {#if appState.filteredRows.length === 0}
        <div class="flex-1 flex flex-col items-center justify-center text-zinc-400 py-12 bg-white">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-12 h-12 mb-3 opacity-60 text-zinc-300 animate-pulse">
            <path stroke-linecap="round" stroke-linejoin="round" d="M20.25 7.5l-.625 10.632a2.25 2.25 0 01-2.247 2.118H6.622a2.25 2.25 0 01-2.247-2.118L3.75 7.5m8.25 3v6.75m0 0l-3-3m3 3l3-3M3.375 7.5h17.25c.621 0 1.125-.504 1.125-1.125v-1.5c0-.621-.504-1.125-1.125-1.125H3.375c-.621 0-1.125.504-1.125 1.125v1.5c0 .621.504 1.125 1.125 1.125z" />
          </svg>
          <span class="text-sm font-medium">No se encontraron registros</span>
        </div>
      {:else}
        <div class="absolute top-0 left-0 w-full" style="height: {totalSize}px;">
          {#each virtualItems as virtualRow (virtualRow.index)}
            {@const row = appState.filteredRows[virtualRow.index]}
            {#if row}
              <div
                class="absolute top-0 left-0 w-full grid border-b border-zinc-100 hover:bg-zinc-50/50 transition-all text-zinc-700 text-xs align-middle items-center"
                style="height: {virtualRow.size}px; transform: translateY({virtualRow.start}px); grid-template-columns: repeat({appState.columns.length}, minmax(180px, 1fr));"
              >
                {#each appState.columns as col}
                  <div class="p-3 border-r border-zinc-100 last:border-r-0 truncate max-w-full select-text" title={row[col]}>
                    {#if row[col] === null || row[col] === undefined}
                      <span class="text-zinc-450 italic font-sans">null</span>
                    {:else if typeof row[col] === 'number'}
                      <span class="font-mono text-zinc-900 font-medium">
                        {row[col].toLocaleString('es-ES', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                      </span>
                    {:else}
                      {row[col]}
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>
