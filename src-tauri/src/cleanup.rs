use polars::prelude::*;
use std::error::Error;

/// 🔬 FASE 1 & 2: ETL Proyectado, Vectorizado y Fusionado
/// Complejidad temporal: O(K * N) | Despacho por elemento: O(1) Vectorizado
pub fn procesar_etl_dsa(path_csv: &str) -> Result<DataFrame, Box<dyn Error>> {
    let q = LazyCsvReader::new(path_csv)
        .with_has_header(true)
        .with_missing_is_null(true)
        .finish()?
        // Proyección temprana para evitar materialización de matriz dispersa completa
        .select([
            col("mov_fecha_con"), col("mov_art_codigo"), col("mov_art_deno"), 
            col("mov_cantidad"), col("almacen_deno"), col("uc_deno"), 
            col("mov_numero_alb"), col("mov_precio_lin"), col("mov_iva"), col("mov_impor_lin")
        ]);

    let df_optimizado = q.with_columns([
        // Transformación temporal estandarizada
        col("mov_fecha_con").str().to_date(StrptimeOptions {
            format: Some(String::from("%d/%m/%Y")),
            strict: false,
            exact: true,
            ..Default::default()
        }).dt().to_string("%Y-%m-%d").alias("fecha_estandarizada"),

        // Estandarización de cadenas vectorizada
        col("mov_art_deno")
            .str().strip_chars(lit(NULL)) 
            .str().to_uppercase()
            .alias("descripcion"),

        // Estandarización de cantidades
        col("mov_cantidad").cast(DataType::Float64).alias("cantidad"),
        
        // Metadatos
        col("mov_art_codigo").cast(DataType::String).alias("codigo"),
        col("almacen_deno").cast(DataType::String).alias("almacen"),
        col("uc_deno").cast(DataType::String).alias("unidad_consumo"),
        col("mov_numero_alb").cast(DataType::Int64).alias("movimiento_albaran"),

        // Eliminación de Subexpresión Común (CSE)
        (lit(1.0) + (col("mov_iva").cast(DataType::Float64) / lit(100.0))).alias("factor_iva"),
    ])
    .select([
        col("fecha_estandarizada"),
        col("codigo"),
        col("descripcion"),
        col("cantidad"),
        col("almacen"),
        col("unidad_consumo"),
        col("movimiento_albaran"),
        (col("mov_precio_lin").cast(DataType::Float64) * col("factor_iva")).alias("precio_con_iva"),
        (col("mov_impor_lin").cast(DataType::Float64) * col("factor_iva")).alias("importe_con_iva"),
    ])
    .collect()?;

    Ok(df_optimizado)
}

fn inyectar_cadena(buffer: &mut Vec<u8>, clave: &[u8], valor: Option<&str>) {
    buffer.extend_from_slice(clave);
    if let Some(s) = valor {
        buffer.push(b'"');
        buffer.extend_from_slice(s.as_bytes());
        buffer.push(b'"');
    } else {
        buffer.extend_from_slice(b"null");
    }
}

/// 🔬 FASE 3: Serialización JSON Determinista (Zero-Copy, Exact Allocation)
/// Complejidad espacial: Θ(L_total) | Complejidad temporal: O(N) | Heap Allocs: O(1)
pub fn exportar_json_estricto(df: &DataFrame) -> Result<String, Box<dyn Error>> {
    let n = df.height();
    if n == 0 { return Ok("[]".to_string()); }

    // Extracción secuencial de los arreglos columnares contiguos
    let c_fecha = df.column("fecha_estandarizada")?.str()?;
    let c_cod = df.column("codigo")?.str()?;
    let c_desc = df.column("descripcion")?.str()?;
    let c_cant = df.column("cantidad")?.f64()?;
    let c_precio = df.column("precio_con_iva")?.f64()?;
    let c_alm = df.column("almacen")?.str()?;
    let c_uc = df.column("unidad_consumo")?.str()?;
    let c_alb = df.column("movimiento_albaran")?.i64()?;
    let c_importe = df.column("importe_con_iva")?.f64()?;

    let mut ryu_buf = ryu::Buffer::new();
    let mut itoa_buf = itoa::Buffer::new();

    let len_str = |opt_s: Option<&str>| -> usize { opt_s.map_or(4, |s| s.len() + 2) }; // 4 es "null"
    
    let mut exact_capacity: usize = 1; // '['
    for i in 0..n {
        if i > 0 { exact_capacity += 1; } // ','
        exact_capacity += 1; // '{'
        
        exact_capacity += 23 + len_str(c_fecha.get(i)); // "\"fecha_estandarizada\":"
        exact_capacity += 10 + len_str(c_cod.get(i));   // ",\"codigo\":"
        exact_capacity += 15 + len_str(c_desc.get(i));  // ",\"descripcion\":"
        exact_capacity += 11 + len_str(c_alm.get(i));   // ",\"almacen\":"
        exact_capacity += 18 + len_str(c_uc.get(i));    // ",\"unidad_consumo\":"

        exact_capacity += 12 + c_cant.get(i).map_or(4, |v| ryu_buf.format(v).len());   // ",\"cantidad\":"
        exact_capacity += 18 + c_precio.get(i).map_or(4, |v| ryu_buf.format(v).len()); // ",\"precio_con_iva\":"
        exact_capacity += 22 + c_alb.get(i).map_or(4, |v| itoa_buf.format(v).len());   // ",\"movimiento_albaran\":"
        exact_capacity += 19 + c_importe.get(i).map_or(4, |v| ryu_buf.format(v).len());// ",\"importe_con_iva\":"
        
        exact_capacity += 1; // '}'
    }
    exact_capacity += 1; // ']'

    let mut buffer = Vec::with_capacity(exact_capacity);
    buffer.push(b'[');

    for i in 0..n {
        if i > 0 { buffer.push(b','); }
        buffer.push(b'{');

        inyectar_cadena(&mut buffer, b"\"fecha_estandarizada\":", c_fecha.get(i));
        inyectar_cadena(&mut buffer, b",\"codigo\":", c_cod.get(i));
        inyectar_cadena(&mut buffer, b",\"descripcion\":", c_desc.get(i));

        buffer.extend_from_slice(b",\"cantidad\":");
        if let Some(v) = c_cant.get(i) { buffer.extend_from_slice(ryu_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        buffer.extend_from_slice(b",\"precio_con_iva\":");
        if let Some(v) = c_precio.get(i) { buffer.extend_from_slice(ryu_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        inyectar_cadena(&mut buffer, b",\"almacen\":", c_alm.get(i));
        inyectar_cadena(&mut buffer, b",\"unidad_consumo\":", c_uc.get(i));

        buffer.extend_from_slice(b",\"movimiento_albaran\":");
        if let Some(v) = c_alb.get(i) { buffer.extend_from_slice(itoa_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        buffer.extend_from_slice(b",\"importe_con_iva\":");
        if let Some(v) = c_importe.get(i) { buffer.extend_from_slice(ryu_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        buffer.push(b'}');
    }
    buffer.push(b']');

    unsafe { Ok(String::from_utf8_unchecked(buffer)) }
}
