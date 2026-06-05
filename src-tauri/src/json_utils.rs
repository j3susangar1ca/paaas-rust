#[inline(always)]
pub fn format_hex_byte(byte: u8) -> [u8; 2] {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    [HEX[(byte >> 4) as usize], HEX[(byte & 0x0F) as usize]]
}

/// Escapado de cadenas altamente optimizado usando copias en bloque (memcpy/SIMD).
/// Supera exponencialmente la inserción byte a byte (`buf.push`).
pub fn write_json_escaped(buf: &mut Vec<u8>, s: &str) {
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
