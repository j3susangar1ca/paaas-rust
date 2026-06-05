use polars::prelude::*;
use std::error::Error;

/// 🔬 FASE 1 & 2: ETL Unificado y Proyectado (Vectorización Nativa Arrow/SIMD)
pub fn procesar_etl_inventario_elite(path_csv: &str) -> Result<DataFrame, Box<dyn Error>> {
    let df_optimizado = LazyCsvReader::new(path_csv)
        .with_has_header(true)
        .with_missing_is_null(true)
        .finish()?
        .select([
            col("Act").str().to_uppercase().eq(lit("SÍ")).alias("activo"),
            col("Código").cast(DataType::String).alias("codigo"),
            col("Nombre")
                .str().strip_chars(lit(NULL))
                .str().to_uppercase()
                .alias("descripcion"),
            col("Existencias").cast(DataType::Int64).alias("existencias"),
            col("Stock Máximo").cast(DataType::Int64).alias("stock_max"),
            col("Precio Medio").cast(DataType::Float64).alias("precio_unitario"),
            col("Importe con IVA").cast(DataType::Float64).alias("importe"),
        ])
        .collect()?;

    Ok(df_optimizado)
}



#[inline(always)]
fn format_hex_byte(byte: u8) -> [u8; 2] {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    [HEX[(byte >> 4) as usize], HEX[(byte & 0x0F) as usize]]
}

/// Escapado de cadenas altamente optimizado usando copias en bloque (memcpy/SIMD).
/// Supera exponencialmente la inserción byte a byte (`buf.push`).
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
            0x00..=0x1F => b"\\u00", // Secuencia base para control
            _ => continue,
        };

        // Copia masiva de los caracteres seguros previos
        buf.extend_from_slice(&bytes[last_idx..i]);
        buf.extend_from_slice(escape_seq);

        if b <= 0x1F && b != b'\x08' && b != b'\x0C' && b != b'\n' && b != b'\r' && b != b'\t' {
            buf.extend_from_slice(&format_hex_byte(b));
        }
        
        last_idx = i + 1;
    }
    // Escribir el resto de la cadena
    buf.extend_from_slice(&bytes[last_idx..]);
    buf.push(b'"');
}


/// 🔬 FASE 3: Serialización Híbrida (Zero-Copy Numérico, Allocación Segura Pasada Única)
pub fn exportar_inventario_elite_definitivo(df: &DataFrame) -> Result<String, Box<dyn Error>> {
    let n = df.height();
    if n == 0 { return Ok("[]".to_string()); }

    let c_activo = df.column("activo")?.bool()?;
    let c_cod = df.column("codigo")?.str()?;
    let c_desc = df.column("descripcion")?.str()?;
    let c_ext = df.column("existencias")?.i64()?;
    let c_stk = df.column("stock_max")?.i64()?;
    let c_prec = df.column("precio_unitario")?.f64()?;
    let c_imp = df.column("importe")?.f64()?;

    let mut it_activo = c_activo.into_iter();
    let mut it_cod = c_cod.into_iter();
    let mut it_desc = c_desc.into_iter();
    let mut it_ext = c_ext.into_iter();
    let mut it_stk = c_stk.into_iter();
    let mut it_prec = c_prec.into_iter();
    let mut it_imp = c_imp.into_iter();

    // Buffers independientes reusables para prevenir aliasing y zero-allocation.
    let mut itoa_ext = itoa::Buffer::new();
    let mut itoa_stk = itoa::Buffer::new();
    let mut ryu_prec = ryu::Buffer::new();
    let mut ryu_imp  = ryu::Buffer::new();

    // Pre-alocación generosa (estimación de ~200 bytes por fila) para evitar re-alocaciones en el heap
    let mut buffer: Vec<u8> = Vec::with_capacity(n * 200);
    buffer.push(b'[');

    for i in 0..n {
        if i > 0 { buffer.push(b','); }
        buffer.push(b'{');

        let v_activo = it_activo.next().flatten();
        let v_cod = it_cod.next().flatten();
        let v_desc = it_desc.next().flatten();
        let v_ext = it_ext.next().flatten();
        let v_stk = it_stk.next().flatten();
        let v_prec = it_prec.next().flatten();
        let v_imp = it_imp.next().flatten();

        // activo
        buffer.extend_from_slice(b"\"activo\":");
        match v_activo {
            Some(true) => buffer.extend_from_slice(b"true"),
            Some(false) => buffer.extend_from_slice(b"false"),
            None => buffer.extend_from_slice(b"null"),
        }

        // codigo (escapado veloz vía bloque)
        buffer.extend_from_slice(b",\"codigo\":");
        if let Some(s) = v_cod { write_json_escaped(&mut buffer, s); } else { buffer.extend_from_slice(b"null"); }

        // descripcion (escapado veloz vía bloque)
        buffer.extend_from_slice(b",\"descripcion\":");
        if let Some(s) = v_desc { write_json_escaped(&mut buffer, s); } else { buffer.extend_from_slice(b"null"); }

        // existencias (cálculo O(1) de itoa UNA sola vez)
        buffer.extend_from_slice(b",\"existencias\":");
        if let Some(v) = v_ext { buffer.extend_from_slice(itoa_ext.format(v).as_bytes()); } else { buffer.extend_from_slice(b"null"); }

        // stock_max (cálculo O(1) de itoa UNA sola vez)
        buffer.extend_from_slice(b",\"stock_max\":");
        if let Some(v) = v_stk { buffer.extend_from_slice(itoa_stk.format(v).as_bytes()); } else { buffer.extend_from_slice(b"null"); }

        // precio_unitario (cálculo complejo de Ryū UNA sola vez)
        buffer.extend_from_slice(b",\"precio_unitario\":");
        if let Some(v) = v_prec { buffer.extend_from_slice(ryu_prec.format(v).as_bytes()); } else { buffer.extend_from_slice(b"null"); }

        // importe (cálculo complejo de Ryū UNA sola vez)
        buffer.extend_from_slice(b",\"importe\":");
        if let Some(v) = v_imp { buffer.extend_from_slice(ryu_imp.format(v).as_bytes()); } else { buffer.extend_from_slice(b"null"); }

        buffer.push(b'}');
    }
    buffer.push(b']');

    let json_payload = String::from_utf8(buffer).map_err(|e| Box::new(e) as Box<dyn Error>)?;
    Ok(json_payload)
}