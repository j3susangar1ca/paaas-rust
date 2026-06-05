# DataEngine Desktop Workspace

Un entorno de escritorio de alto rendimiento impulsado por **Tauri v2**, **Svelte 5** (con Runes reactivos), y el motor analítico **Rust Polars** para realizar tareas de ETL, visualización y filtrado rápido sobre archivos CSV masivos con latencia ultra baja.

---

## 🔬 Arquitectura del Sistema

La aplicación está diseñada para desacoplar las tareas visuales y de renderizado (del renderer HTML/JS) de los procesamientos pesados de datos (en el core Rust).

```
 ┌──────────────────────────────────────┐
 │          Frontend Svelte 5           │
 └──────────────────┬───────────────────┘
                    │  Invocación de Comandos Tauri
                    ▼
 ┌──────────────────────────────────────┐
 │         Tauri v2 App Wrapper         │
 └──────────────────┬───────────────────┘
                    │
                    ▼
 ┌──────────────────────────────────────┐
 │      Rust Backend (Polars Engine)    │
 └──────────────────────────────────────┘
   - Carga & Validación CSV
   - Estandarización / ETL Vectorizado
   - Caché Global de DataFrame (RwLock)
   - Decimación LTTB para Gráficos WebGL
   - Serializador JSON de pasada única
   - Exportación directa CSV
```

---

## 🛠️ Flujos y Módulos de Procesamiento (ETL)

El backend de Rust contiene los siguientes módulos especializados:

*   **`json_utils.rs`**: Contiene la lógica de codificación y escape JSON altamente optimizada mediante copias en bloque para evitar asignaciones repetidas de heap.
*   **`etl_movimientos.rs`**: Procesa el esquema de movimientos. Realiza proyecciones tempranas, conversiones y estandarización temporal a formato `YYYY-MM-DD`, cálculo del factor de IVA y serialización determinista.
*   **`etl_inventario.rs`**: Procesa el esquema de inventario consolidado. Realiza filtrado booleano de actividad, estandariza cadenas y formatea campos numéricos.
*   **`lib.rs`**: Coordina el ciclo de vida del estado, gestiona la caché global en un `RwLock<Option<CachedData>>` que previene cruces accidentales de esquemas, y realiza validaciones de seguridad (como Path Traversal y firmas de extensión).

---

## 💾 Comandos Tauri Disponibles

*   `procesar_csv_command`: Ejecuta el pipeline ETL sobre el CSV y carga el DataFrame procesado en la caché global. Devuelve la serialización JSON del dataset.
*   `filtrar_datos_command`: Filtra reactivamente el DataFrame en caché utilizando una expresión `contains_literal` de Polars de forma paralela en las columnas de tipo texto, sin afectar columnas numéricas.
*   `obtener_datos_decimados`: Implementa el algoritmo de decimación LTTB (Largest Triangle Three Buckets) en Rust. Filtra valores `NaN`, ordena los datos temporal o numéricamente mediante `f64::total_cmp`, y devuelve exactamente los puntos indicados (por defecto 2000) para graficar mediante WebGL (Plotly) en el frontend.
*   `exportar_csv_filtrado_command`: Exporta el DataFrame filtrado actual directamente a disco en formato CSV utilizando el `CsvWriter` nativo de Polars (O(N) de alta velocidad), reduciendo el consumo de memoria del navegador.

---

## 🛡️ Seguridad y Confiabilidad

*   **Content Security Policy (CSP)**: Cuenta con una CSP estricta habilitada en `tauri.conf.json` para bloquear inyecciones XSS maliciosas.
*   **Validación de Rutas**: Las escrituras de archivos se validan mediante la función `validate_save_path` que previene el Path Traversal (`..`), verifica la extensión `.csv` y deniega la escritura en directorios raíz del sistema (`/etc/`, `/bin/`, `/var/`, etc.).
*   **Gestión de Hilos Segura**: La caché global usa `RwLock` y sus bloqueos se recuperan mediante `unwrap_or_else(|e| e.into_inner())` para evitar pánicos irrecuperables en Tauri en caso de locks envenenados (poisoned lock).

---

## 🚀 Guía de Desarrollo

### Requisitos Previos
*   **Rust**: Versión estable de `rustc` y `cargo` (1.70+ recomendado).
*   **Node.js**: Versión 18 o superior con gestor de paquetes `npm`.

### Ejecutar en Desarrollo
Instala las dependencias y lanza el entorno interactivo de Tauri:
```bash
npm install
npm run tauri dev
```

### Ejecutar Tests del Backend
Para ejecutar las pruebas unitarias que validan el algoritmo LTTB, el formateador JSON, y las conversiones de fecha:
```bash
cd src-tauri
cargo test
```

### Compilar para Producción
Genera el binario optimizado compilado a nivel 3 (`opt-level = 3`, `LTO` habilitado):
```bash
npm run tauri build
```
