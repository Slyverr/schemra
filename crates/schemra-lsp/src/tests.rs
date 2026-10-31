use lsp_types::Position;

use crate::positions::{byte_offset_at_position, range_for_bytes};

#[test]
fn byte_ranges_count_utf16_units_after_non_bmp_text() {
    let source = "visual \"🌐\" {\n  node";
    let start = source.find("node").expect("source contains node");
    let range = range_for_bytes(source, start, start + "node".len());

    assert_eq!(range.start, Position::new(1, 2));
    assert_eq!(range.end, Position::new(1, 6));
}

#[test]
fn positions_inside_a_surrogate_pair_map_to_its_start() {
    let source = "🌐node";

    assert_eq!(byte_offset_at_position(source, Position::new(0, 1)), 0);
}
