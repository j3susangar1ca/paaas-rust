<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { appState } from '$lib/store/dataState.svelte';
  import VirtualTable from '$lib/components/VirtualTable.svelte';
  import ChartComponent from '$lib/components/ChartComponent.svelte';

  async function loadCSV() {
    if (!appState.loadedPath.trim()) return;
    appState.isLoading = true;
    appState.cpuStatus = 'Procesando ETL en Rust Polars...';
    try {
      const start = performance.now();
      // Invoke Tauri command for fast ETL and exact JSON stringification, passing file type
      const json = await invoke<string>('procesar_csv_command', { 
        path: appState.loadedPath.trim(),
        tipo: appState.schemaType
      });
      const end = performance.now();
      
      const elapsed = ((end - start) / 1000).toFixed(3);
      appState.cpuStatus = `ETL completado en ${elapsed}s (Polars Engine)`;
      
      const parsed = JSON.parse(json);
      appState.rawRows = parsed;
      
      // Auto-configure axis variables based on loaded columns and schema
      if (parsed.length > 0) {
        const cols = Object.keys(parsed[0]);
        if (appState.schemaType === 'inventario') {
          appState.selectedXCol = cols.includes('codigo') ? 'codigo' : cols[0];
          appState.selectedYCol = cols.includes('importe') ? 'importe' : (cols.includes('existencias') ? 'existencias' : cols[1]);
        } else {
          appState.selectedXCol = cols.includes('fecha_estandarizada') ? 'fecha_estandarizada' : cols[0];
          appState.selectedYCol = cols.includes('importe_con_iva') ? 'importe_con_iva' : (cols.includes('cantidad') ? 'cantidad' : cols[1]);
        }
      }
    } catch (err: any) {
      console.error(err);
      appState.cpuStatus = `Error: ${err}`;
      alert(`Error al cargar el CSV:\n${err}`);
    } finally {
      appState.isLoading = false;
    }
  }

  function exportFilteredData() {
    if (appState.filteredRows.length === 0) return;
    
    appState.cpuStatus = 'Exportando CSV filtrado...';
    const cols = appState.columns;
    const headers = cols.join(',');
    const rows = appState.filteredRows.map((row: any) => 
      cols.map((col: string) => {
        const val = row[col];
        if (val === null || val === undefined) return '';
        const strVal = String(val);
        // Escape standard CSV cells
        if (strVal.includes(',') || strVal.includes('"') || strVal.includes('\n')) {
          return `"${strVal.replace(/"/g, '""')}"`;
        }
        return strVal;
      }).join(',')
    );
    
    const csvContent = [headers, ...rows].join('\n');
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.setAttribute("href", url);
    link.setAttribute("download", "data_procesada.csv");
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
    appState.cpuStatus = `CSV exportado (${appState.filteredRows.length} registros)`;
  }

  // Handle Ctrl+O and Ctrl+S hotkeys for native desktop feel
  onMount(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {
        e.preventDefault();
        const input = document.getElementById('csv-path-input') as HTMLInputElement | null;
        if (input) input.focus();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
        e.preventDefault();
        exportFilteredData();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

<div class="grid h-screen grid-rows-[56px_1fr_40px] bg-slate-950 text-slate-100 overflow-hidden font-sans">
  <!-- Fixed Custom Header with Drag Region -->
  <header class="data-tauri-drag-region select-none h-14 border-b border-slate-800 bg-slate-900/80 backdrop-blur-md px-5 flex items-center justify-between z-50">
    <div class="flex items-center gap-3">
      <!-- Premium Glass Icon Logo -->
      <div class="w-8 h-8 rounded-lg bg-gradient-to-tr from-emerald-500 to-teal-400 flex items-center justify-center shadow-lg shadow-emerald-500/20">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2.5" stroke="currentColor" class="w-5 h-5 text-slate-950">
          <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" />
        </svg>
      </div>
      <div>
        <h1 class="font-bold text-sm tracking-wide text-slate-100">DataEngine <span class="text-xs text-emerald-400 font-normal">v1.2</span></h1>
        <p class="text-[10px] text-slate-500 leading-none">Polars-Powered Desktop ETL</p>
      </div>
    </div>

    <!-- Active Indicators and Stats -->
    <div class="flex items-center gap-4">
      {#if appState.rawRows.length > 0}
        <div class="flex items-center gap-2">
          <span class="text-[10px] text-slate-500 uppercase tracking-wider">Registros cargados</span>
          <span class="badge bg-slate-800 border-slate-700 text-emerald-400 font-mono font-semibold px-2.5 py-1">
            {appState.rawRows.length.toLocaleString('es-ES')}
          </span>
        </div>
        {#if appState.filteredRows.length !== appState.rawRows.length}
          <div class="flex items-center gap-2">
            <span class="text-[10px] text-slate-500 uppercase tracking-wider">Filtrados</span>
            <span class="badge bg-slate-800 border-slate-700 text-teal-400 font-mono font-semibold px-2.5 py-1">
              {appState.filteredRows.length.toLocaleString('es-ES')}
            </span>
          </div>
        {/if}
      {/if}
    </div>
  </header>

  <!-- Main Grid Area: Sidebar + Body Workspace -->
  <main class="grid grid-cols-[280px_1fr] overflow-hidden bg-slate-950">
    
    <!-- Sidebar Control Center -->
    <aside class="border-r border-slate-900 bg-slate-900/30 p-5 flex flex-col justify-between overflow-y-auto select-none">
      
      <!-- Top controls block -->
      <div class="flex flex-col gap-6">
        
        <!-- 1. CSV Loader Section -->
        <div class="flex flex-col gap-3">
          <div class="flex flex-col gap-1">
            <label for="schema-select" class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Esquema / Tipo de Datos</label>
            <select
              id="schema-select"
              bind:value={appState.schemaType}
              class="select select-sm select-bordered w-full bg-slate-950 border-slate-800 text-xs text-slate-300"
            >
              <option value="movimientos">Movimientos (cleanup.rs)</option>
              <option value="inventario">Inventario (cleanup2.rs)</option>
            </select>
          </div>

          <div class="flex flex-col gap-1.5">
            <label for="csv-path-input" class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Ruta de Archivo CSV</label>
            <input
              id="csv-path-input"
              type="text"
              placeholder="/ruta/al/archivo.csv"
              bind:value={appState.loadedPath}
              class="input input-sm input-bordered w-full bg-slate-950 border-slate-800 text-xs focus:border-emerald-500 transition-all font-mono"
            />
          </div>

          <button
            onclick={loadCSV}
            disabled={appState.isLoading || !appState.loadedPath.trim()}
            class="btn btn-sm btn-primary bg-emerald-500 hover:bg-emerald-600 disabled:bg-slate-800 disabled:text-slate-600 border-none text-slate-950 font-bold transition-all"
          >
            {#if appState.isLoading}
              <span class="loading loading-spinner loading-xs"></span>
              Procesando...
            {:else}
              Cargar CSV
            {/if}
          </button>
        </div>

        {#if appState.rawRows.length > 0}
          <div class="divider border-slate-900 my-1"></div>

          <!-- 2. Workspace View Selector -->
          <div class="flex flex-col gap-2">
            <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Vista de Espacio</span>
            <div class="grid grid-cols-2 gap-2">
              <button
                onclick={() => appState.activeTab = 'tabla'}
                class="btn btn-sm border {appState.activeTab === 'tabla' ? 'bg-emerald-500/10 border-emerald-500/50 text-emerald-400' : 'bg-slate-950 border-slate-850 text-slate-400'} font-semibold transition-all hover:bg-slate-800"
              >
                Tabla
              </button>
              <button
                onclick={() => appState.activeTab = 'grafico'}
                class="btn btn-sm border {appState.activeTab === 'grafico' ? 'bg-emerald-500/10 border-emerald-500/50 text-emerald-400' : 'bg-slate-950 border-slate-850 text-slate-400'} font-semibold transition-all hover:bg-slate-800"
              >
                Gráfico
              </button>
            </div>
          </div>

          <!-- 3. Dynamic Sidebar Options based on Active View -->
          {#if appState.activeTab === 'tabla'}
            <!-- Filters for Virtual Table -->
            <div class="flex flex-col gap-2 transition-all">
              <label for="search-input" class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Filtrar Registros</label>
              <input
                id="search-input"
                type="text"
                placeholder="Buscar término..."
                bind:value={appState.filterQuery}
                class="input input-sm input-bordered w-full bg-slate-950 border-slate-800 text-xs focus:border-emerald-500"
              />
              <span class="text-[10px] text-slate-500">Búsqueda rápida en todas las columnas</span>
            </div>
          {:else if appState.activeTab === 'grafico'}
            <!-- Axis Columns Selectors for WebGL Plotly Chart -->
            <div class="flex flex-col gap-4 transition-all">
              <div class="flex flex-col gap-1.5">
                <label for="x-axis-select" class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Variable Eje X</label>
                <select
                  id="x-axis-select"
                  bind:value={appState.selectedXCol}
                  class="select select-sm select-bordered w-full bg-slate-950 border-slate-800 text-xs text-slate-300"
                >
                  {#each appState.columns as col}
                    <option value={col}>{col}</option>
                  {/each}
                </select>
              </div>

              <div class="flex flex-col gap-1.5">
                <label for="y-axis-select" class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Variable Eje Y</label>
                <select
                  id="y-axis-select"
                  bind:value={appState.selectedYCol}
                  class="select select-sm select-bordered w-full bg-slate-950 border-slate-800 text-xs text-slate-300"
                >
                  {#each appState.columns as col}
                    <!-- Prioritize numerical values for Y -->
                    <option value={col}>{col}</option>
                  {/each}
                </select>
              </div>
            </div>
          {/if}

          <div class="divider border-slate-900 my-1"></div>

          <!-- 4. CSV Exporter -->
          <div class="flex flex-col gap-2">
            <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Operaciones de Datos</span>
            <button
              onclick={exportFilteredData}
              disabled={appState.filteredRows.length === 0}
              class="btn btn-sm btn-outline border-slate-800 hover:border-emerald-500 hover:bg-emerald-500/10 hover:text-emerald-400 text-slate-300 text-xs transition-all w-full"
            >
              Exportar CSV Filtrado
            </button>
          </div>
        {/if}
      </div>

      <!-- Bottom shortcuts legend block -->
      <div class="p-3 bg-slate-950/60 border border-slate-900 rounded-lg flex flex-col gap-1.5 text-[10px] text-slate-500">
        <span class="font-bold text-slate-400 uppercase tracking-wider mb-0.5">Atajos de Teclado</span>
        <div class="flex items-center justify-between">
          <span>Enfocar ruta CSV</span>
          <kbd class="kbd kbd-xs bg-slate-900 border-slate-800 text-slate-400 font-mono">Ctrl+O</kbd>
        </div>
        <div class="flex items-center justify-between">
          <span>Exportar Tabla</span>
          <kbd class="kbd kbd-xs bg-slate-900 border-slate-800 text-slate-400 font-mono">Ctrl+S</kbd>
        </div>
      </div>
    </aside>

    <!-- Workspace Main Panel -->
    <section class="p-5 overflow-hidden flex flex-col bg-slate-950">
      <div class="flex-1 w-full h-full overflow-hidden">
        {#if appState.isLoading}
          <!-- Gorgeous skeleton loader during heavy process -->
          <div class="w-full h-full flex flex-col gap-4 p-5 bg-slate-900/40 border border-slate-900 rounded-xl animate-pulse">
            <div class="h-8 bg-slate-800 rounded-lg w-1/4"></div>
            <div class="h-10 bg-slate-800 rounded-lg w-full"></div>
            <div class="flex-1 bg-slate-800 rounded-lg w-full"></div>
            <div class="h-8 bg-slate-800 rounded-lg w-1/3"></div>
          </div>
        {:else if appState.rawRows.length === 0}
          <!-- Interactive premium Welcome Dashboard empty state -->
          <div class="w-full h-full flex flex-col items-center justify-center border border-dashed border-slate-800/80 rounded-2xl bg-gradient-to-b from-slate-900/10 to-slate-950/20 px-8 text-center relative overflow-hidden">
            <div class="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(16,185,129,0.03),transparent_60%)]"></div>
            
            <div class="w-20 h-20 rounded-2xl bg-slate-900 border border-slate-800 flex items-center justify-center shadow-2xl mb-6 text-emerald-400 relative z-10">
              <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-12 h-12">
                <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m6.75 12H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
              </svg>
            </div>
            
            <h2 class="text-xl font-bold text-slate-200 tracking-tight mb-2 relative z-10">Carga un Archivo CSV de Datos</h2>
            <p class="text-sm text-slate-500 max-w-md mb-6 leading-relaxed relative z-10">
              Copia la ruta absoluta de tu archivo de movimientos CSV y presiona <strong class="text-emerald-500 font-semibold">Cargar CSV</strong>. Nuestro motor Rust Polars se encargará del procesamiento ETL vectorizado.
            </p>

            <div class="w-full max-w-lg bg-slate-900/80 border border-slate-800 rounded-xl p-4 flex items-center gap-3 relative z-10 text-left">
              <div class="w-8 h-8 rounded-lg bg-slate-950 flex items-center justify-center border border-slate-850">
                <span class="text-xs text-slate-400">💡</span>
              </div>
              <p class="text-xs text-slate-400 font-medium">
                Pega la ruta del CSV del proyecto para probar el pipeline, por ejemplo:<br/>
                <code class="text-emerald-400 select-all font-mono font-bold block mt-1">/home/jesuslangarica/paaas-rust/movimientos.csv</code>
              </p>
            </div>
          </div>
        {:else}
          <!-- Render components dynamically based on active tab Selection -->
          {#if appState.activeTab === 'tabla'}
            <VirtualTable />
          {:else if appState.activeTab === 'grafico'}
            <ChartComponent />
          {/if}
        {/if}
      </div>
    </section>
  </main>

  <!-- Fixed App Footer -->
  <footer class="h-10 border-t border-slate-900 bg-slate-900/60 px-5 flex items-center justify-between text-xs text-slate-500 select-none">
    <div class="flex items-center gap-4">
      <span class="flex items-center gap-1.5">
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
        Motor Rust: <strong class="text-slate-400 font-medium">{appState.cpuStatus}</strong>
      </span>
    </div>
    <div class="flex items-center gap-3">
      <span>Tauri v2 + Svelte 5 + Polars ETL</span>
      <span class="text-slate-700">|</span>
      <span>2026 Desktop Workspace</span>
    </div>
  </footer>
</div>
