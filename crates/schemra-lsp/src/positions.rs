use lsp_types::{Position, Range};

pub(crate) fn range_for_bytes(source: &str, start: usize, end: usize) -> Range {
    Range {
        start: position_at_byte(source, start),
        end: position_at_byte(source, end),
    }
}

fn position_at_byte(source: &str, byte_offset: usize) -> Position {
    let mut end = byte_offset.min(source.len());
    while !source.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let before = &source[..end];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let line_start = before.rfind('\n').map_or(0, |offset| offset + 1);
    let line_prefix = before[line_start..]
        .strip_suffix('\r')
        .unwrap_or(&before[line_start..]);
    Position::new(line, line_prefix.encode_utf16().count() as u32)
}

pub(crate) fn byte_offset_at_position(source: &str, position: Position) -> usize {
    let mut line_start = 0;
    for _ in 0..position.line {
        let Some(newline) = source[line_start..].find('\n') else {
            return source.len();
        };
        line_start += newline + 1;
    }
    let rest = &source[line_start..];
    let line_end = rest.find('\n').unwrap_or(rest.len());
    let mut line = &rest[..line_end];
    if let Some(without_carriage_return) = line.strip_suffix('\r') {
        line = without_carriage_return;
    }

    let target_units = position.character as usize;
    let mut units = 0;
    for (byte_offset, character) in line.char_indices() {
        if target_units == units {
            return line_start + byte_offset;
        }
        let next_units = units + character.len_utf16();
        if target_units < next_units {
            return line_start + byte_offset;
        }
        units = next_units;
    }
    line_start + line.len()
}
