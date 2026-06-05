export interface MovimientoRow {
  fecha_estandarizada: string | null;
  codigo: string;
  descripcion: string;
  cantidad: number | null;
  precio_con_iva: number | null;
  almacen: string;
  unidad_consumo: string;
  movimiento_albaran: number | null;
  importe_con_iva: number | null;
  [key: string]: any; // Fallback index signature for dynamic table cells
}

export interface InventarioRow {
  activo: boolean | null;
  codigo: string;
  descripcion: string;
  existencias: number | null;
  stock_max: number | null;
  precio_unitario: number | null;
  importe: number | null;
  [key: string]: any; // Fallback index signature for dynamic table cells
}

export type RowType = MovimientoRow | InventarioRow;

class AppState {
  rawRows = $state<RowType[]>([]);
  isLoading = $state(false);
  filterQuery = $state("");
  schemaType = $state<'movimientos' | 'inventario'>('movimientos');
  activeTab = $state<'tabla' | 'grafico'>('tabla');
  selectedXCol = $state<string>('fecha_estandarizada');
  selectedYCol = $state<string>('importe_con_iva');
  loadedPath = $state<string>('');
  cpuStatus = $state<string>('Inactivo');

  filteredRows = $state<RowType[]>([]);

  // Helper to extract unique columns from the loaded rows
  get columns() {
    if (this.rawRows.length === 0 || !this.rawRows[0]) return [];
    return Object.keys(this.rawRows[0]);
  }
}

export const appState = new AppState();
