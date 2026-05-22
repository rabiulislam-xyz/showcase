/// Parse a human size like "92.6 MB", "1.2 GB", "512 kB" into bytes (1024-based).
/// Returns None if unparseable.
pub fn parse_human_size(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, unit) = s.split_at(
        s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len()),
    );
    let value: f64 = num.trim().parse().ok()?;
    let mult: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" | "kib" | "k" => 1024.0,
        "mb" | "mib" | "m" => 1024.0 * 1024.0,
        "gb" | "gib" | "g" => 1024.0 * 1024.0 * 1024.0,
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
}
