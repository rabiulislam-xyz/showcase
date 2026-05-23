/// Parse a human size like "92.6 MB", "1.2 GB", "512 kB" into bytes (1024-based).
///
/// Returns None if:
/// - the string is empty or unparseable,
/// - the numeric part is negative,
/// - the numeric part contains thousands commas (e.g. "1,024 MB"),
/// - there is trailing junk after the unit (e.g. "10 MB extra").
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Reject thousands-comma numbers before any further parsing.
    if s.contains(',') {
        return None;
    }

    let split_at = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num_part, unit_part) = s.split_at(split_at);

    let value: f64 = num_part.trim().parse().ok()?;

    // Reject negative values.
    if value < 0.0 {
        return None;
    }

    let unit = unit_part.trim();

    // Reject trailing junk: unit must be a recognised token with no whitespace
    // or extra characters after it.
    let mult: f64 = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" | "kib" | "k" => 1024.0,
        "mb" | "mib" | "m" => 1024.0 * 1024.0,
        "gb" | "gib" | "g" => 1024.0 * 1024.0 * 1024.0,
        "tb" | "tib" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * mult) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_units() {
        assert_eq!(parse_human_size("92.6 MB"), Some((92.6 * 1024.0 * 1024.0) as u64));
        assert_eq!(parse_human_size("1.0 GB"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_human_size("512 kB"), Some(512 * 1024));
        assert_eq!(parse_human_size(""), None);
        assert_eq!(parse_human_size("garbage"), None);
    }

    #[test]
    fn parses_tb_and_tib() {
        let tb: u64 = 1024 * 1024 * 1024 * 1024;
        assert_eq!(parse_human_size("1 TB"), Some(tb));
        assert_eq!(parse_human_size("1 tb"), Some(tb));
        assert_eq!(parse_human_size("1 TiB"), Some(tb));
        assert_eq!(parse_human_size("2.5 TB"), Some((2.5 * tb as f64) as u64));
    }

    #[test]
    fn rejects_negative_values() {
        assert_eq!(parse_human_size("-5 MB"), None);
        assert_eq!(parse_human_size("-1 GB"), None);
        assert_eq!(parse_human_size("-0.5 kB"), None);
    }

    #[test]
    fn rejects_thousands_comma_numbers() {
        assert_eq!(parse_human_size("1,024 MB"), None);
        assert_eq!(parse_human_size("1,024,576 kB"), None);
    }

    #[test]
    fn rejects_trailing_junk_after_unit() {
        assert_eq!(parse_human_size("10 MB extra"), None);
        assert_eq!(parse_human_size("5 GB x"), None);
        // A plain number with no unit is valid (treated as bytes).
        assert_eq!(parse_human_size("1024"), Some(1024));
    }
}
