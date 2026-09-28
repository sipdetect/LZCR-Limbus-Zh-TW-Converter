pub fn normalize_release_date(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    if value.len() >= 10 {
        let date = &value[..10];
        if date.chars().all(|c| c.is_ascii_digit() || c == '-') {
            return Some(date.to_string());
        }
    }
    None
}

pub fn release_date_from_tag(tag: &str) -> Option<String> {
    let digits: String = tag.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    let d = &digits[..8];
    Some(format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]))
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    if bytes == 0 {
        return "0 B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0usize;
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{size:.2} {}", UNITS[unit_index])
}

pub fn ellipsize(input: &str, max_chars: usize) -> String {
    let total = input.chars().count();
    if total <= max_chars {
        return input.to_string();
    }

    let take_tail = max_chars.saturating_sub(3);
    let tail: String = input
        .chars()
        .skip(total.saturating_sub(take_tail))
        .collect();
    format!("...{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_release_date_parses_iso_prefix() {
        assert_eq!(
            normalize_release_date(Some("2024-03-15T12:00:00Z")),
            Some("2024-03-15".to_string())
        );
    }

    #[test]
    fn release_date_from_tag_parses_digits() {
        assert_eq!(
            release_date_from_tag("v20240315-release"),
            Some("2024-03-15".to_string())
        );
    }

    #[test]
    fn ellipsize_truncates_long_paths() {
        let result = ellipsize("C:\\very\\long\\path\\to\\file.json", 20);
        assert!(result.starts_with("..."));
        assert!(result.chars().count() <= 20);
    }
}