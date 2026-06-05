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



#[inline(always)]
fn format_hex_byte(byte: u8) -> [u8; 2] {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    [HEX[(byte >> 4) as usize], HEX[(byte & 0x0F) as usize]]
}

fn write_json_escaped(buf: &mut Vec<u8>, s: &str) {
    buf.push(b'"');
    let bytes = s.as_bytes();
    let mut last_idx = 0;

    for (i, &b) in bytes.iter().enumerate() {
        let escape_seq: &[u8] = match b {
            b'"' => b"\\\"",
            b'\\' => b"\\\\",
            b'\x08' => b"\\b",
            b'\x0C' => b"\\f",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            b'\t' => b"\\t",
            0x00..=0x1F => b"\\u00",
            _ => continue,
        };

        buf.extend_from_slice(&bytes[last_idx..i]);
        buf.extend_from_slice(escape_seq);

        if b <= 0x1F && b != b'\x08' && b != b'\x0C' && b != b'\n' && b != b'\r' && b != b'\t' {
            buf.extend_from_slice(&format_hex_byte(b));
        }
        
        last_idx = i + 1;
    }
    buf.extend_from_slice(&bytes[last_idx..]);
    buf.push(b'"');
}

fn inyectar_cadena(buffer: &mut Vec<u8>, clave: &[u8], valor: Option<&str>) {
    buffer.extend_from_slice(clave);
    if let Some(s) = valor {
        write_json_escaped(buffer, s);
    } else {
        buffer.extend_from_slice(b"null");
    }
}

/// 🔬 FASE 3: Serialización JSON Determinista (Zero-Copy, Pasada Única por Iteradores Contiguos)
/// Complejidad espacial: Θ(L_total) | Complejidad temporal: O(N) | Heap Allocs: O(1)
pub fn exportar_json_estricto(df: &DataFrame) -> Result<String, Box<dyn Error>> {
    let n = df.height();
    if n == 0 { return Ok("[]".to_string()); }

    // Extracción de los arrays columnares
    let c_fecha = df.column("fecha_estandarizada")?.str()?;
    let c_cod = df.column("codigo")?.str()?;
    let c_desc = df.column("descripcion")?.str()?;
    let c_cant = df.column("cantidad")?.f64()?;
    let c_precio = df.column("precio_con_iva")?.f64()?;
    let c_alm = df.column("almacen")?.str()?;
    let c_uc = df.column("unidad_consumo")?.str()?;
    let c_alb = df.column("movimiento_albaran")?.i64()?;
    let c_importe = df.column("importe_con_iva")?.f64()?;

    // Iteradores contiguos sobre las columnas en memoria Arrow
    let mut it_fecha = c_fecha.into_iter();
    let mut it_cod = c_cod.into_iter();
    let mut it_desc = c_desc.into_iter();
    let mut it_cant = c_cant.into_iter();
    let mut it_precio = c_precio.into_iter();
    let mut it_alm = c_alm.into_iter();
    let mut it_uc = c_uc.into_iter();
    let mut it_alb = c_alb.into_iter();
    let mut it_importe = c_importe.into_iter();

    // Buffers numéricos reusables
    let mut ryu_buf = ryu::Buffer::new();
    let mut itoa_buf = itoa::Buffer::new();

    // Pre-alocación generosa (estimación de ~200 bytes por fila) para evitar re-alocaciones en el heap
    let mut buffer = Vec::with_capacity(n * 200);
    buffer.push(b'[');

    for i in 0..n {
        if i > 0 { buffer.push(b','); }
        buffer.push(b'{');

        let v_fecha = it_fecha.next().flatten();
        let v_cod = it_cod.next().flatten();
        let v_desc = it_desc.next().flatten();
        let v_cant = it_cant.next().flatten();
        let v_precio = it_precio.next().flatten();
        let v_alm = it_alm.next().flatten();
        let v_uc = it_uc.next().flatten();
        let v_alb = it_alb.next().flatten();
        let v_importe = it_importe.next().flatten();

        inyectar_cadena(&mut buffer, b"\"fecha_estandarizada\":", v_fecha);
        inyectar_cadena(&mut buffer, b",\"codigo\":", v_cod);
        inyectar_cadena(&mut buffer, b",\"descripcion\":", v_desc);

        buffer.extend_from_slice(b",\"cantidad\":");
        if let Some(v) = v_cant { buffer.extend_from_slice(ryu_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        buffer.extend_from_slice(b",\"precio_con_iva\":");
        if let Some(v) = v_precio { buffer.extend_from_slice(ryu_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        inyectar_cadena(&mut buffer, b",\"almacen\":", v_alm);
        inyectar_cadena(&mut buffer, b",\"unidad_consumo\":", v_uc);

        buffer.extend_from_slice(b",\"movimiento_albaran\":");
        if let Some(v) = v_alb { buffer.extend_from_slice(itoa_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        buffer.extend_from_slice(b",\"importe_con_iva\":");
        if let Some(v) = v_importe { buffer.extend_from_slice(ryu_buf.format(v).as_bytes()); } 
        else { buffer.extend_from_slice(b"null"); }

        buffer.push(b'}');
    }
    buffer.push(b']');

    String::from_utf8(buffer).map_err(|e| Box::new(e) as Box<dyn Error>)
}
