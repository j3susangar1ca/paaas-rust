class AppState {
  rawRows = $state<any[]>([]);
  isLoading = $state(false);
  filterQuery = $state("");
  schemaType = $state<'movimientos' | 'inventario'>('movimientos');
  activeTab = $state<'tabla' | 'grafico'>('tabla');
  selectedXCol = $state<string>('fecha_estandarizada');
  selectedYCol = $state<string>('importe_con_iva');
  loadedPath = $state<string>('');
  cpuStatus = $state<string>('Inactivo');

  filteredRows = $state<any[]>([]);

  // Helper to extract unique columns from the loaded rows
  get columns() {
    if (this.rawRows.length === 0 || !this.rawRows[0]) return [];
    return Object.keys(this.rawRows[0]);
  }
}

export const appState = new AppState();
