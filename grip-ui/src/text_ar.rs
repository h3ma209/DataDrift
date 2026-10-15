//! Arabic display for engines without OpenType shaping (egui / ab_glyph).
//!
//! 1. Map letters → presentation forms (`arabic_reshaper`) so they join.
//! 2. Reorder to visual run (`unicode-bidi`) for LTR UI.

use arabic_reshaper::arabic_reshape;
use unicode_bidi::BidiInfo;

pub fn needs_arabic(s: &str) -> bool {
    s.chars().any(is_arabic_char)
}

fn is_arabic_char(c: char) -> bool {
    matches!(
        c,
        '\u{0600}'..='\u{06FF}'
            | '\u{0750}'..='\u{077F}'
            | '\u{08A0}'..='\u{08FF}'
            | '\u{FB50}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFF}'
    )
}

/// Shape + visual reorder for grid / labels. No-op when no Arabic.
pub fn for_display(s: &str) -> String {
    if s.is_empty() || !needs_arabic(s) {
        return s.to_string();
    }
    let reshaped = arabic_reshape(s);
    let info = BidiInfo::new(&reshaped, None);
    let mut out = String::with_capacity(reshaped.len());
    for para in &info.paragraphs {
        let line = para.range.clone();
        out.push_str(&info.reorder_line(para, line));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reshape_changes_presentation() {
        let logical = "احمد";
        let visual = for_display(logical);
        assert_ne!(visual, logical, "should map to presentation forms");
        assert!(needs_arabic(logical));
        // Still non-empty printable
        assert!(!visual.is_empty());
    }

    #[test]
    fn latin_passthrough() {
        assert_eq!(for_display("Hemn"), "Hemn");
        assert_eq!(for_display("600725"), "600725");
    }
}
