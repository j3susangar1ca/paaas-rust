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



        // codigo (escapado veloz vía bloque)
        buffer.extend_from_slice(b",\"codigo\":");
        if let Some(s) = v_cod { crate::json_utils::write_json_escaped(&mut buffer, s); } else { buffer.extend_from_slice(b"null"); }
 
        // descripcion (escapado veloz vía bloque)
        buffer.extend_from_slice(b",\"descripcion\":");
        if let Some(s) = v_desc { crate::json_utils::write_json_escaped(&mut buffer, s); } else { buffer.extend_from_slice(b"null"); }

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