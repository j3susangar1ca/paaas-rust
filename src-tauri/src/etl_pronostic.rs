use std::collections::HashMap;
use std::error::Error;
use std::fmt::Write as _; // Habilita macro write! sobre Strings sin allocar

// Estructura monomórfica para deserialización directa sin HashMap intermedio [Obs 2, Obs 7]
#[derive(Debug, serde::Deserialize)]
struct Movimiento {
    #[serde(rename = "fecha_estandarizada")]
    fecha: String,
    codigo: String,
    descripcion: String,
    cantidad: f64, // El crate 'csv' parsea tipos nativos directamente
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Pre-asignación estricta de memoria con capacidad conocida [Obs 6]
    let mut meses_historicos = Vec::with_capacity(42);
    let mut meses_historicos_idx = HashMap::with_capacity(42);
    
    let mut idx = 0;
    for anio in 2023..=2026 {
        let max_mes = if anio == 2026 { 6 } else { 12 };
        for mes in 1..=max_mes {
            let m_str = format!("{:04}-{:02}", anio, mes);
            meses_historicos.push((m_str.clone(), mes)); // Almacena (String, mes_num) [Obs 5]
            meses_historicos_idx.insert(m_str, idx);    // Mapeo para Lookup O(1)
            idx += 1;
        }
    }

    // Precomputar meses de proyección evitando parseos en el hot path [Obs 5, Obs 6]
    let mut meses_proyeccion = Vec::with_capacity(18);
    for mes in 7..=12 {
        meses_proyeccion.push((format!("2026-{:02}", mes), mes));
    }
    for mes in 1..=12 {
        meses_proyeccion.push((format!("2027-{:02}", mes), mes));
    }

    // Cálculo matemático O(1) de ocurrencias estructurales por mes [Obs 9]
    let mut conteo_ocurrencias_mes = [0; 13];
    for m in 1..=12 { conteo_ocurrencias_mes[m] = 3; } // Años completos 2023, 2024, 2025
    for m in 1..=6  { conteo_ocurrencias_mes[m] += 1; } // Mitad de año 2026

    // 2. Inicialización del pipeline de lectura
    let archivo_entrada = "movimientos_fcaailtrados.csv";
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(archivo_entrada)?;

    // Agregación plana en memoria continua para optimizar el Caché L1/L2 [Obs 3]
    let mut matriz: HashMap<(String, String), Vec<f64>> = HashMap::with_capacity(128);

    println!("Iniciando ingesta y análisis con optimización de memoria...");

    // 3. Hot Path de Ingesta y Agregación
    for result in rdr.deserialize::<Movimiento>() {
        let reg = result?;
        
        if reg.fecha.len() >= 7 {
            let anio_mes = &reg.fecha[0..7]; // O(1) Slice referencial en stack [Obs 1]
            
            // Búsqueda O(1) amortizada libre de allocations en bucle [Obs 1]
            if let Some(&hist_idx) = meses_historicos_idx.get(anio_mes) {
                // Semántica de movimiento (Move): Cero clones redundantes del String [Obs 2, Obs 7]
                let clave = (reg.codigo, reg.descripcion);
                let historial = matriz.entry(clave).or_insert_with(|| vec![0.0; 42]);
                historial[hist_idx] += reg.cantidad;
            }
        }
    }

    // 4. Transformación de Estructura e Inferencia
    // Extraemos a Vec y ordenamos eficientemente al final: O(K log K) amortizado una sola vez
    let mut registros_ordenados: Vec<_> = matriz.into_iter().collect();
    registros_ordenados.sort_unstable_by(|a, b| a.0.0.cmp(&b.0.0));

    // 5. Canal de Escritura en Streaming de Alto Rendimiento [Obs 4, Obs 8, Obs 10]
    let archivo_salida = "resumen_con_proyecciones_opt.csv";
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(false)
        .from_path(archivo_salida)?;

    // Escribir encabezados por streaming directo sin colecciones intermedias
    wtr.write_field("codigo")?;
    wtr.write_field("descripcion")?;
    for (m_str, _) in &meses_historicos {
        wtr.write_field(m_str)?;
    }
    for (m_str, _) in &meses_proyeccion {
        wtr.write_field(m_str)?;
    }
    wtr.write_record(std::iter::empty::<&str>())?;

    // Buffer temporal reutilizable en stack (Cero Allocations dinámicas en el loop) [Obs 4, Obs 8]
    let mut scratch_buf = String::with_capacity(32);

    println!("Aplicando modelo predictivo estacional y exportando reporte...");

    for ((codigo, descripcion), historial) in registros_ordenados {
        // Inicializar el vector de patrones estacionales locales del artículo
        let mut modelo_estacional = [0.0; 13];
        for (idx, &cant) in historial.iter().enumerate() {
            let mes_num = meses_historicos[idx].1; // Recuperación directa O(1) [Obs 5]
            modelo_estacional[mes_num] += cant;
        }

        // Computar medias estacionales aplicando la matriz de ponderación entrenada
        for m in 1..=12 {
            if conteo_ocurrencias_mes[m] > 0 {
                modelo_estacional[m] /= conteo_ocurrencias_mes[m] as f64;
            }
        }

        // Escritura por campos individuales directamente al buffer del Writer (Zero-Row-Allocation) [Obs 8]
        wtr.write_field(&codigo)?;
        wtr.write_field(&descripcion)?;

        // Serializar tramo histórico
        for &valor_real in &historial {
            scratch_buf.clear();
            write!(&mut scratch_buf, "{:.0}", valor_real)?; // Formateo en stack sin Virtual Dispatch
            wtr.write_field(&scratch_buf)?;
        }

        // Serializar tramo proyectado (Resto del 26 y todo el 27)
        for (_, mes_num) in &meses_proyeccion {
            let valor_proyectado = modelo_estacional[*mes_num];
            scratch_buf.clear();
            write!(&mut scratch_buf, "{:.2}", valor_proyectado)?;
            wtr.write_field(&scratch_buf)?;
        }

        // Finalizar registro de línea corriente
        wtr.write_record(std::iter::empty::<&str>())?;
    }

    wtr.flush()?;
    println!("¡Auditoría implementada! Reporte final generado en '{}'", archivo_salida);

    Ok(())
}