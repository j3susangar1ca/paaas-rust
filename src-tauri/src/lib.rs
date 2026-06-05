// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod cleanup;
mod cleanup2;

use cleanup::{procesar_etl_dsa, exportar_json_estricto};
use cleanup2::{procesar_etl_inventario_elite, exportar_inventario_elite_definitivo};
use polars::prelude::*;
use std::sync::{RwLock, OnceLock};

// Global cache for processed DataFrame to avoid re-reading the CSV for chart queries.
static DATAFRAME_CACHE: OnceLock<RwLock<Option<DataFrame>>> = OnceLock::new();

fn get_dataframe_cache() -> &'static RwLock<Option<DataFrame>> {
    DATAFRAME_CACHE.get_or_init(|| RwLock::new(None))
}

// Howard Hinnant's algorithm to calculate days since 1970-01-01 for any Gregorian date.
fn date_to_days(y: i32, m: i32, d: i32) -> i32 {
    let y = y - (m <= 2) as i32;
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let y_in_era = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = y_in_era * 365 + y_in_era / 4 - y_in_era / 100 + doy;
    era * 146097 + doe - 719468
}

fn parse_date_to_epoch_days(date_str: &str) -> Option<f64> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y = parts[0].parse::<i32>().ok()?;
    let m = parts[1].parse::<i32>().ok()?;
    let d = parts[2].parse::<i32>().ok()?;
    Some(date_to_days(y, m, d) as f64)
}

// LTTB (Largest Triangle Three Buckets) downsampling algorithm.
pub fn lttb(data: &[(f64, f64)], threshold: usize) -> Vec<(f64, f64)> {
    let data_len = data.len();
    if threshold >= data_len || threshold < 3 {
        return data.to_vec();
    }

    let mut sampled = Vec::with_capacity(threshold);
    let bucket_size = (data_len - 2) as f64 / (threshold - 2) as f64;

    sampled.push(data[0]);

    let mut a = 0; // index of the selected point from the previous bucket

    for i in 0..threshold - 2 {
        // Range of the next bucket to calculate the barycenter (avg)
        let next_start = ((i + 1) as f64 * bucket_size).floor() as usize + 1;
        let next_end = (((i + 2) as f64 * bucket_size).floor() as usize + 1).min(data_len);

        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let count = (next_end - next_start) as f64;
        if count > 0.0 {
            for j in next_start..next_end {
                avg_x += data[j].0;
                avg_y += data[j].1;
            }
            avg_x /= count;
            avg_y /= count;
        }

        // Range of the current bucket
        let curr_start = (i as f64 * bucket_size).floor() as usize + 1;
        let curr_end = (((i + 1) as f64 * bucket_size).floor() as usize + 1).min(data_len);

        let mut max_area = -1.0;
        let mut next_a = curr_start;

        let p_a = data[a];

        for j in curr_start..curr_end {
            let p_curr = data[j];
            let area = 0.5 * (
                p_a.0 * (p_curr.1 - avg_y) +
                p_curr.0 * (avg_y - p_a.1) +
                avg_x * (p_a.1 - p_curr.1)
            ).abs();

            if area > max_area {
                max_area = area;
                next_a = j;
            }
        }

        sampled.push(data[next_a]);
        a = next_a;
    }

    sampled.push(data[data_len - 1]);
    sampled
}

#[tauri::command]
fn procesar_csv_command(path: String, tipo: String) -> Result<String, String> {
    if tipo == "inventario" {
        let df = procesar_etl_inventario_elite(&path).map_err(|e| e.to_string())?;
        
        // Save to global cache
        let mut cache = get_dataframe_cache().write().unwrap_or_else(|e| e.into_inner());
        *cache = Some(df.clone());

        exportar_inventario_elite_definitivo(&df).map_err(|e| e.to_string())
    } else {
        let df = procesar_etl_dsa(&path).map_err(|e| e.to_string())?;
        
        // Save to global cache
        let mut cache = get_dataframe_cache().write().unwrap_or_else(|e| e.into_inner());
        *cache = Some(df.clone());

        exportar_json_estricto(&df).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn obtener_datos_decimados(x_col: String, y_col: String, n_buckets: Option<usize>) -> Result<Vec<(f64, f64)>, String> {
    let cache = get_dataframe_cache().read().unwrap_or_else(|e| e.into_inner());
    let df = match &*cache {
        Some(df) => df,
        None => return Err("No hay datos cargados en memoria. Cargue un archivo CSV primero.".to_string()),
    };

    let x_series = df.column(&x_col).map_err(|e| e.to_string())?;
    let y_series = df.column(&y_col).map_err(|e| e.to_string())?;

    // Keep dynamic casts owned in local scope so borrows can live as long as iterators
    let x_cast;
    let x_ref = if matches!(x_series.dtype(), DataType::String | DataType::Int64 | DataType::Float64) {
        x_series
    } else {
        x_cast = x_series.cast(&DataType::Float64).map_err(|e| e.to_string())?;
        &x_cast
    };

    let y_cast;
    let y_ref = if matches!(y_series.dtype(), DataType::String | DataType::Int64 | DataType::Float64) {
        y_series
    } else {
        y_cast = y_series.cast(&DataType::Float64).map_err(|e| e.to_string())?;
        &y_cast
    };

    let mut points = Vec::with_capacity(df.height());

    let x_iter = match x_ref.dtype() {
        DataType::String => {
            let ca = x_ref.str().map_err(|e| e.to_string())?;
            Box::new(ca.into_iter().map(|opt_s| {
                opt_s.and_then(|s| parse_date_to_epoch_days(s).or_else(|| s.parse::<f64>().ok()))
            })) as Box<dyn Iterator<Item = Option<f64>>>
        }
        DataType::Int64 => {
            let ca = x_ref.i64().map_err(|e| e.to_string())?;
            Box::new(ca.into_iter().map(|opt_v| opt_v.map(|v| v as f64)))
        }
        _ => {
            let ca = x_ref.f64().map_err(|e| e.to_string())?;
            Box::new(ca.into_iter())
        }
    };

    let y_iter = match y_ref.dtype() {
        DataType::String => {
            let ca = y_ref.str().map_err(|e| e.to_string())?;
            Box::new(ca.into_iter().map(|opt_s| {
                opt_s.and_then(|s| parse_date_to_epoch_days(s).or_else(|| s.parse::<f64>().ok()))
            })) as Box<dyn Iterator<Item = Option<f64>>>
        }
        DataType::Int64 => {
            let ca = y_ref.i64().map_err(|e| e.to_string())?;
            Box::new(ca.into_iter().map(|opt_v| opt_v.map(|v| v as f64)))
        }
        _ => {
            let ca = y_ref.f64().map_err(|e| e.to_string())?;
            Box::new(ca.into_iter())
        }
    };

    for (x_opt, y_opt) in x_iter.zip(y_iter) {
        if let (Some(x), Some(y)) = (x_opt, y_opt) {
            points.push((x, y));
        }
    }

    // Sort by x coordinate chronologically for LTTB to work correctly
    points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let threshold = n_buckets.unwrap_or(2000);
    let decimated = lttb(&points, threshold);
    Ok(decimated)
}

#[tauri::command]
fn filtrar_datos_command(query: String, tipo: String) -> Result<String, String> {
    let cache = get_dataframe_cache().read().unwrap_or_else(|e| e.into_inner());
    let df = match &*cache {
        Some(df) => df,
        None => return Ok("[]".to_string()),
    };

    let q = query.trim();
    if q.is_empty() {
        if tipo == "inventario" {
            return exportar_inventario_elite_definitivo(df).map_err(|e| e.to_string());
        } else {
            return exportar_json_estricto(df).map_err(|e| e.to_string());
        }
    }

    let q_upper = q.to_uppercase();
    let cols = df.get_column_names();
    let mut predicate = lit(false);
    for col_name in cols {
        let cond = col(col_name)
            .cast(DataType::String)
            .str()
            .to_uppercase()
            .str()
            .contains(lit(q_upper.clone()), true);
        predicate = predicate.or(cond);
    }

    let filtered_df = df.clone().lazy().filter(predicate).collect().map_err(|e| e.to_string())?;

    if tipo == "inventario" {
        exportar_inventario_elite_definitivo(&filtered_df).map_err(|e| e.to_string())
    } else {
        exportar_json_estricto(&filtered_df).map_err(|e| e.to_string())
    }
}

fn validate_save_path(path_str: &str) -> Result<(), String> {
    let path = std::path::Path::new(path_str);
    
    // 1. Check extension is .csv (case insensitive)
    if let Some(ext) = path.extension() {
        if ext.to_string_lossy().to_lowercase() != "csv" {
            return Err("La extensión del archivo debe ser .csv".to_string());
        }
    } else {
        return Err("El archivo no tiene una extensión válida".to_string());
    }
    
    // 2. Prevent path traversal by checking for directory traversal components (..)
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Componente de ruta inválido (path traversal detectado)".to_string());
        }
    }
    
    // 3. Prevent writing to critical system directories if absolute path
    if path.is_absolute() {
        let str_path = path.to_string_lossy();
        let bad_prefixes = [
            "/etc/", "/var/", "/bin/", "/usr/", "/lib/", "/boot/", "/sys/", "/proc/", "/dev/"
        ];
        for prefix in bad_prefixes {
            if str_path.starts_with(prefix) {
                return Err(format!("No está permitido escribir en directorios del sistema como '{}'", prefix));
            }
        }
    }

    Ok(())
}

#[tauri::command]
fn guardar_archivo_csv(path: String, content: String) -> Result<(), String> {
    validate_save_path(&path)?;
    std::fs::write(&path, content).map_err(|e| format!("Error al guardar el archivo: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            procesar_csv_command,
            obtener_datos_decimados,
            guardar_archivo_csv,
            filtrar_datos_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

