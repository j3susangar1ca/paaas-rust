<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { appState } from '$lib/store/dataState.svelte';

  let chartEl = $state<HTMLDivElement | null>(null);
  let chartLoading = $state(false);
  let chartError = $state('');

  // Lazy-loaded reference to the Plotly library
  let Plotly: any = null;
  let resizeObserver: ResizeObserver | null = null;

  async function renderChart() {
    if (!chartEl || appState.rawRows.length === 0) return;
    chartLoading = true;
    chartError = '';
    
    try {
      // Lazy load Plotly on demand to keep the initial application bundle lightweight
      if (!Plotly) {
        appState.cpuStatus = 'Cargando módulo de gráficos...';
        const PlotlyModule = await import('plotly.js-dist-min');
        Plotly = PlotlyModule.default;
      }

      appState.cpuStatus = 'Procesando decimación LTTB...';
      
      // Invoke Rust downsampling command with Tauri v2 parameters
      const points = await invoke<[number, number][]>('obtener_datos_decimados', {
        xCol: appState.selectedXCol,
        yCol: appState.selectedYCol,
        nBuckets: 2500
      });

      appState.cpuStatus = 'ETL + Decimación en Rust completada';

      if (points.length === 0) {
        chartError = 'No hay datos válidos para graficar con las columnas seleccionadas.';
        return;
      }

      // Check if X column represents dates to cast Unix timestamps properly
      const isDate = appState.selectedXCol === 'fecha_estandarizada' || appState.selectedXCol === 'mov_fecha_con';
      
      // If date, map epoch days to Date objects
      const x = points.map(p => isDate ? new Date(p[0] * 86400 * 1000) : p[0]);
      const y = points.map(p => p[1]);

      const trace = {
        x,
        y,
        type: 'scattergl', // Native WebGL acceleration
        mode: 'lines+markers',
        line: { color: '#0071e3', width: 1.5 }, // Apple System Blue
        marker: { color: '#005bb7', size: 3 }, // Apple Dark Blue
        name: `${appState.selectedYCol}`
      };

      const layout = {
        paper_bgcolor: 'rgba(0,0,0,0)',
        plot_bgcolor: 'rgba(0,0,0,0)',
        font: { color: '#1d1d1f', family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
        margin: { t: 30, r: 20, l: 60, b: 50 },
        hovermode: 'closest',
        dragmode: 'zoom',
        xaxis: {
          gridcolor: 'rgba(0, 0, 0, 0.05)', // light grid
          linecolor: '#d4d4d8', // Zinc 300
          type: isDate ? 'date' : 'linear',
          title: { text: appState.selectedXCol, font: { size: 10, color: '#86868b' } },
          tickfont: { size: 9, color: '#86868b' }
        },
        yaxis: {
          gridcolor: 'rgba(0, 0, 0, 0.05)', // light grid
          linecolor: '#d4d4d8', // Zinc 300
          title: { text: appState.selectedYCol, font: { size: 10, color: '#86868b' } },
          tickfont: { size: 9, color: '#86868b' }
        }
      };

      const config = {
        responsive: true,
        displayModeBar: true,
        displaylogo: false,
        modeBarButtonsToRemove: ['select2d', 'lasso2d']
      };

      Plotly.newPlot(chartEl, [trace], layout, config);
    } catch (err: any) {
      console.error('Error rendering chart:', err);
      chartError = `Error al generar el gráfico: ${err.message || err}`;
    } finally {
      chartLoading = false;
    }
  }

  // Handle dynamic resizing through ResizeObserver to support fluid window adjustments
  onMount(() => {
    resizeObserver = new ResizeObserver(() => {
      if (chartEl && Plotly) {
        Plotly.Plots.resize(chartEl);
      }
    });

    if (chartEl) {
      resizeObserver.observe(chartEl);
    }

    return () => {
      if (resizeObserver) {
        resizeObserver.disconnect();
      }
    };
  });

  // Reactive effect when active tab, data, or column selections change
  $effect(() => {
    const x = appState.selectedXCol;
    const y = appState.selectedYCol;
    const tab = appState.activeTab;
    const count = appState.rawRows.length;

    if (tab === 'grafico' && count > 0) {
      const timer = setTimeout(renderChart, 80);
      return () => clearTimeout(timer);
    }
  });
</script>

<div class="h-full w-full flex flex-col bg-white border border-zinc-200 rounded-xl overflow-hidden shadow-sm p-4">
  <div class="flex items-center justify-between mb-4">
    <h3 class="text-xs font-bold text-zinc-400 uppercase tracking-wider">Vista Gráfica Decimada (LTTB Rust Engine)</h3>
    <div class="flex items-center gap-3">
      {#if chartLoading}
        <span class="loading loading-ring loading-md text-blue-500"></span>
      {/if}
      <span class="text-[10px] text-zinc-550 bg-zinc-50 px-2 py-0.5 rounded border border-zinc-200 shadow-sm font-medium">
        Puntos decimados: 2,500
      </span>
    </div>
  </div>

  <div class="flex-1 w-full relative min-h-[300px]">
    {#if chartLoading}
      <div class="absolute inset-0 bg-white/80 backdrop-blur-sm flex flex-col items-center justify-center rounded-lg z-10 transition-all duration-300">
        <span class="loading loading-ring loading-lg text-blue-500 mb-3 animate-pulse"></span>
        <span class="text-xs text-zinc-700 font-semibold tracking-wider uppercase">Generando gráfico WebGL...</span>
        <span class="text-[10px] text-zinc-400 mt-1 font-mono">{appState.cpuStatus}</span>
      </div>
    {/if}

    {#if chartError}
      <div class="absolute inset-0 flex flex-col items-center justify-center text-red-500 bg-zinc-50/50 p-4 text-center rounded-lg border border-zinc-200 shadow-sm">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-10 h-10 mb-2 opacity-80">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m0-10.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.75c0 5.592 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.57-.598-3.75h-.152c-3.196 0-6.1-1.249-8.25-3.286zm0 13.036h.008v.008H12v-.008z" />
        </svg>
        <p class="text-sm font-semibold">{chartError}</p>
      </div>
    {:else}
      <div bind:this={chartEl} class="w-full h-full"></div>
    {/if}
  </div>
</div>
