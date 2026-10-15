use mysql::Value;

pub fn value_to_string(v: Value) -> String {
    match v {
        Value::NULL => "NULL".into(),
        Value::Bytes(b) => decode_bytes(&b),
        Value::Int(n) => n.to_string(),
        Value::UInt(n) => n.to_string(),
        Value::Float(f) => format_float(f as f64),
        Value::Double(f) => format_float(f),
        Value::Date(y, m, d, h, mi, s, micros) => {
            if h == 0 && mi == 0 && s == 0 && micros == 0 {
                format!("{y:04}-{m:02}-{d:02}")
            } else if micros == 0 {
                format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
            } else {
                format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}.{micros:06}")
            }
        }
        Value::Time(neg, d, h, mi, s, micros) => {
            let sign = if neg { "-" } else { "" };
            if micros == 0 {
                format!("{sign}{d} {h:02}:{mi:02}:{s:02}")
            } else {
                format!("{sign}{d} {h:02}:{mi:02}:{s:02}.{micros:06}")
            }
        }
    }
}

/// UTF-8 first; fall back to lossy UTF-8 (MySQL utf8mb4 text arrives as Bytes).
fn decode_bytes(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(b).into_owned(),
    }
}

fn format_float(f: f64) -> String {
    if !f.is_finite() {
        return f.to_string();
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{:.0}", f)
    } else {
        format!("{}", f)
    }
}

pub fn row_cell(row: &mysql::Row, index: usize) -> String {
    row.get::<Option<Value>, _>(index)
        .flatten()
        .map(value_to_string)
        .unwrap_or_else(|| "NULL".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_arabic_bytes() {
        let s = value_to_string(Value::Bytes("احمد".as_bytes().to_vec()));
        assert_eq!(s, "احمد");
    }

    #[test]
    fn ints_plain() {
        assert_eq!(value_to_string(Value::Int(600725)), "600725");
    }
}
