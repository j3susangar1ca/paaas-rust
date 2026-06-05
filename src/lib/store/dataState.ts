class AppState {
  rawRows = $state<any[]>([]);
  isLoading = $state(false);
  filterQuery = $state("");
  activeTab = $state<'tabla' | 'grafico'>('tabla');
  selectedXCol = $state<string>('fecha_estandarizada');
  selectedYCol = $state<string>('importe_con_iva');
  loadedPath = $state<string>('');
  cpuStatus = $state<string>('Inactivo');

  // Derived filtered rows (automatically computed without blocking)
  get filteredRows() {
    const q = this.filterQuery.trim().toLowerCase();
    if (!q) return this.rawRows;
    return this.rawRows.filter(row => 
      Object.values(row).some(v => String(v).toLowerCase().includes(q))
    );
  }

  // Helper to extract unique columns from the loaded rows
  get columns() {
    if (this.rawRows.len === 0 || !this.rawRows[0]) return [];
    return Object.keys(this.rawRows[0]);
  }
}

export const appState = new AppState();
