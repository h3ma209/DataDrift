//! Statement splitting / caret execute (DataGrip Ctrl+Enter map).

/// Split on `;` (trimmed). Empty chunks dropped.
pub fn statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Statement containing byte offset `caret`, or last non-empty if caret past end,
/// or whole trimmed buffer if no `;`.
pub fn statement_at_caret(sql: &str, caret: usize) -> String {
    let caret = caret.min(sql.len());
    let mut start = 0usize;
    for (i, ch) in sql.char_indices() {
        if ch == ';' {
            let chunk = sql[start..i].trim();
            if caret <= i {
                if !chunk.is_empty() {
                    return chunk.to_string();
                }
            }
            start = i + 1;
        }
    }
    let tail = sql[start..].trim();
    if !tail.is_empty() {
        return tail.to_string();
    }
    // caret in trailing whitespace after last `;` → previous statement
    let all = statements(sql);
    all.last().cloned().unwrap_or_default()
}

/// If selection non-empty, use it; else statement at caret.
pub fn execute_target(sql: &str, caret: usize, selection: Option<&str>) -> String {
    if let Some(sel) = selection {
        let t = sel.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    statement_at_caret(sql, caret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_two() {
        let s = statements("SELECT 1; SELECT 2;");
        assert_eq!(s, vec!["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn caret_second() {
        let sql = "SELECT 1;\nSELECT name FROM t;";
        let caret = sql.find("name").unwrap();
        assert_eq!(statement_at_caret(sql, caret), "SELECT name FROM t");
    }

    #[test]
    fn caret_first() {
        let sql = "SELECT 1;\nSELECT 2;";
        assert_eq!(statement_at_caret(sql, 0), "SELECT 1");
    }
}
