/// Accepts project-relative raster paths and bounded base64 raster data URLs.
/// SVG, other URL schemes, and traversal segments are excluded.
pub fn is_safe_image_source(source: &str) -> bool {
    let source = source.trim();
    if let Some(data) = source.strip_prefix("data:image/") {
        let Some((media_type, payload)) = data.split_once(";base64,") else {
            return false;
        };
        if !matches!(media_type, "png" | "jpeg" | "webp" | "gif")
            || payload.is_empty()
            || payload.len() > 12 * 1024 * 1024
            || payload.len() % 4 != 0
        {
            return false;
        }
        let mut padding_started = false;
        let mut padding_count = 0;
        for byte in payload.bytes() {
            if byte == b'=' {
                padding_started = true;
                padding_count += 1;
                if padding_count > 2 {
                    return false;
                }
            } else if byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/') {
                if padding_started {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }
    if source.is_empty()
        || source.starts_with('/')
        || source.contains('\\')
        || source.contains(':')
        || source.contains('?')
        || source.contains('#')
        || source.contains('%')
        || source.chars().any(char::is_control)
    {
        return false;
    }
    let path = source.strip_prefix("./").unwrap_or(source);
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return false;
    }
    let extension = path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif")
}
