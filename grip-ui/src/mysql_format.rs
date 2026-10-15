use mysql::Value;

pub fn value_to_string(v: Value) -> String {
    match v {
        Value::NULL => "NULL".into(),
        Value::Bytes(b) => {
            if let Ok(s) = std::str::from_utf8(&b) {
                s.to_string()
            } else {
                String::from_utf8_lossy(&b).into_owned()
            }
        }
        Value::Int(n) => n.to_string(),
        Value::UInt(n) => n.to_string(),
        Value::Float(f) => format_float(f),
        Value::Double(f) => format_float(f as f32),
        Value::Date(y, m, d, h, mi, s, micros) => {
            if h == 0 && mi == 0 && s == 0 && micros == 0 {
                format!("{y:04}-{m:02}-{d:02}")
            } else {
                format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
            }
        }
        Value::Time(neg, d, h, mi, s, micros) => {
            let sign = if neg { "-" } else { "" };
            format!("{sign}{d} {h:02}:{mi:02}:{s:02}.{micros:06}")
        }
    }
}

fn format_float(f: f32) -> String {
    if f.fract() == 0.0 && f.is_finite() {
        format!("{:.0}", f)
    } else {
        f.to_string()
    }
}

pub fn row_cell(row: &mysql::Row, index: usize) -> String {
    row.get::<Option<Value>, _>(index)
        .flatten()
        .map(value_to_string)
        .unwrap_or_else(|| "NULL".into())
}
