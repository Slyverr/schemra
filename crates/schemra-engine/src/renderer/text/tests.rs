use super::{font_profiles_for_text, node_text_width, wrap_text_lines, wrap_text_runs};
use schemra_scene::{SceneFont, SceneNodeLayout};

#[test]
fn wrapping_uses_glyph_advances_instead_of_character_counts() {
    let narrow_glyphs = wrap_text_lines(SceneFont::Sans, "iiiiiiiiiiii", 80.0, 17.0, 650);
    let wide_glyphs = wrap_text_lines(SceneFont::Sans, "WWWWWWWWWWWW", 80.0, 17.0, 650);

    assert_eq!(narrow_glyphs.len(), 1);
    assert!(wide_glyphs.len() > narrow_glyphs.len());
}

#[test]
fn explicit_blank_lines_are_preserved() {
    let lines = wrap_text_lines(SceneFont::Serif, "first\n\nlast", 220.0, 14.0, 450);

    assert_eq!(lines, ["first", "", "last"]);
}

#[test]
fn bundled_profiles_shape_their_cross_target_fixture_glyphs_without_missing_glyphs() {
    use cosmic_text::{Attrs, Buffer, Family, Metrics, Shaping, Weight};

    for (font, text) in [
        (SceneFont::Sans, "Schemra requests"),
        (SceneFont::Serif, "Crème Δοκιμή Журнал €"),
        (SceneFont::Mono, "Crème Δοκιμή Журнал € →"),
    ] {
        let mut system = super::font_system()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut buffer = Buffer::new(&mut system, Metrics::new(16.0, 16.0));
        buffer.set_size(&mut system, None, None);
        buffer.set_text(
            &mut system,
            text,
            &Attrs::new()
                .family(Family::Name(super::family_name(font)))
                .weight(Weight(450)),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut system, false);

        let glyphs = buffer
            .layout_runs()
            .flat_map(|run| {
                run.glyphs
                    .iter()
                    .map(|glyph| (glyph.glyph_id, glyph.font_id))
            })
            .collect::<Vec<_>>();

        assert!(!glyphs.is_empty(), "{font:?} shaped no glyphs");
        assert!(
            glyphs.iter().all(|(glyph_id, _)| *glyph_id != 0),
            "{font:?} produced a missing glyph while shaping {text:?}"
        );
        assert!(
            glyphs.iter().all(
                |(_, font_id)| system.db().face(*font_id).is_some_and(|face| face
                    .families
                    .iter()
                    .any(|(name, _)| name == super::family_name(font)))
            ),
            "{font:?} fell back to a different bundled family while shaping {text:?}"
        );
    }
}

#[test]
fn representative_missing_glyphs_use_a_bundled_face() {
    for (font, text) in [
        (SceneFont::Sans, "ŉ"),
        (SceneFont::Serif, "→"),
        (SceneFont::Mono, "↖"),
    ] {
        assert!(
            font_profiles_for_text(font, text, 450)
                .into_iter()
                .any(|used| used),
            "expected shaping to select a fallback face for {font:?} {text:?}"
        );
    }
}

#[test]
fn fallback_runs_follow_logical_cluster_ranges() {
    for text in ["Crème →", "A→B"] {
        let line = wrap_text_runs(SceneFont::Serif, text, 320.0, 16.0, 450)
            .into_iter()
            .next()
            .unwrap();
        let Some(runs) = line.runs else {
            panic!("{text:?} unexpectedly required whole-line fallback");
        };
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            text,
            "logical text order for {text:?}"
        );
        assert_eq!(line.text, text);
    }
}

#[test]
fn combining_sequence_is_kept_whole_before_a_fallback_run() {
    let text = "e\u{0301}→";
    let line = wrap_text_runs(SceneFont::Serif, text, 320.0, 16.0, 450)
        .into_iter()
        .next()
        .unwrap();
    let Some(runs) = line.runs else {
        panic!("a boundary after a complete grapheme should be safe");
    };

    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<String>(),
        text
    );
    assert!(runs.iter().all(|run| !run.text.starts_with('\u{0301}')));
}

#[test]
fn bidi_controls_and_rtl_runs_keep_original_logical_text_order() {
    let text = "A\u{200f}→\u{200f}B";
    let line = wrap_text_runs(SceneFont::Serif, text, 320.0, 16.0, 450)
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(line.text, text);
    let Some(runs) = line.runs else {
        panic!("script and bidi boundaries should be safe to serialize");
    };
    assert_eq!(
        runs.iter().map(|run| run.text.as_str()).collect::<String>(),
        text
    );
    assert!(runs.iter().any(|run| run.text.contains('→')));
    assert!(runs.iter().any(|run| run.text.contains('\u{200f}')));
}

#[test]
fn overlapping_clusters_merge_only_when_the_selected_face_matches() {
    use super::{ClusterFont, merge_overlapping_clusters};

    assert_eq!(
        merge_overlapping_clusters(vec![
            ClusterFont {
                start: 0,
                end: 3,
                profile: SceneFont::Serif,
            },
            ClusterFont {
                start: 2,
                end: 4,
                profile: SceneFont::Serif,
            },
        ]),
        Some(vec![ClusterFont {
            start: 0,
            end: 4,
            profile: SceneFont::Serif,
        }])
    );
    assert_eq!(
        merge_overlapping_clusters(vec![
            ClusterFont {
                start: 0,
                end: 3,
                profile: SceneFont::Serif,
            },
            ClusterFont {
                start: 2,
                end: 4,
                profile: SceneFont::Mono,
            },
        ]),
        None
    );
}

#[test]
fn font_boundaries_must_respect_graphemes_and_script_runs() {
    assert!(!super::safe_font_boundary("e\u{0301}→", 1));
    assert!(!super::safe_font_boundary("e\u{0301}→", 2));
    assert!(super::safe_font_boundary("e\u{0301}→", 3));
    assert!(!super::safe_font_boundary("AB", 1));
    assert!(super::safe_font_boundary("AЖ", 1));
}

#[test]
fn node_text_width_accounts_for_layout_and_icon_space() {
    let horizontal_with_icon = node_text_width(252.0, SceneNodeLayout::Horizontal, true, 12.0);
    let vertical_with_icon = node_text_width(252.0, SceneNodeLayout::Vertical, true, 12.0);

    assert_eq!(horizontal_with_icon, 174.0);
    assert_eq!(vertical_with_icon, 216.0);
}
