//! Shared glyph-aware text wrapping for scene layout and SVG output.

use std::{
    collections::BTreeMap,
    sync::{Mutex, OnceLock},
};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, LayoutRun, Metrics, Shaping, Weight, Wrap};
use schemra_scene::{SceneFont, SceneNodeLayout};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

const INTER_FONT: &[u8] = include_bytes!("../assets/fonts/inter-v4.1-subset.ttf");
const SERIF_FONT: &[u8] = include_bytes!("../assets/fonts/noto-serif-v2.015-subset.ttf");
const MONO_FONT: &[u8] = include_bytes!("../assets/fonts/noto-sans-mono-v2.013-subset.ttf");

fn family_name(font: SceneFont) -> &'static str {
    match font {
        SceneFont::Sans => "Inter Variable",
        SceneFont::Serif => "Noto Serif",
        SceneFont::Mono => "Noto Sans Mono",
    }
}

fn font_system() -> &'static Mutex<FontSystem> {
    static FONT_SYSTEM: OnceLock<Mutex<FontSystem>> = OnceLock::new();
    FONT_SYSTEM.get_or_init(|| {
        let mut database = cosmic_text::fontdb::Database::new();
        database.load_font_data(INTER_FONT.to_vec());
        database.load_font_data(SERIF_FONT.to_vec());
        database.load_font_data(MONO_FONT.to_vec());
        Mutex::new(FontSystem::new_with_locale_and_db(
            "en-US".to_owned(),
            database,
        ))
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FontRun {
    pub text: String,
    pub profile: SceneFont,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TextLine {
    pub text: String,
    /// `None` means cluster or shaping boundaries were ambiguous, so the line
    /// must stay one SVG text run and use normal platform fallback.
    pub runs: Option<Vec<FontRun>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ClusterFont {
    start: usize,
    end: usize,
    profile: SceneFont,
}

/// Wraps text using the exact bundled profile face used by the SVG renderer.
///
/// Both scene layout and SVG serialization call this function, so changing a
/// font profile, weight, size, or available text width yields the same lines.
pub fn wrap_text_lines(
    font: SceneFont,
    text: &str,
    max_width: f32,
    font_size: f32,
    font_weight: u16,
) -> Vec<String> {
    wrap_text_runs(font, text, max_width, font_size, font_weight)
        .into_iter()
        .map(|line| line.text)
        .collect()
}

pub(super) fn wrap_text_runs(
    font: SceneFont,
    text: &str,
    max_width: f32,
    font_size: f32,
    font_weight: u16,
) -> Vec<TextLine> {
    if text.is_empty() {
        return vec![TextLine {
            text: String::new(),
            runs: Some(Vec::new()),
        }];
    }
    if !max_width.is_finite() || max_width <= 0.0 || !font_size.is_finite() || font_size <= 0.0 {
        return text.split('\n').map(fallback_line).collect();
    }

    let mut system = font_system()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut buffer = Buffer::new(&mut system, Metrics::new(font_size, font_size));
    buffer.set_wrap(&mut system, Wrap::WordOrGlyph);
    buffer.set_size(&mut system, Some(max_width), None);
    buffer.set_text(
        &mut system,
        text,
        &Attrs::new()
            .family(Family::Name(family_name(font)))
            .weight(Weight(font_weight)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut system, false);

    let lines = buffer
        .layout_runs()
        .map(|run| line_from_layout_run(&system, run, font))
        .collect::<Vec<_>>();

    if lines.is_empty() {
        text.split('\n').map(fallback_line).collect()
    } else {
        lines
    }
}

pub(super) fn unwrapped_text_line(
    font: SceneFont,
    text: &str,
    font_size: f32,
    font_weight: u16,
) -> TextLine {
    if text.is_empty() || text.contains('\n') || !font_size.is_finite() || font_size <= 0.0 {
        return fallback_line(text);
    }
    let mut system = font_system()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut buffer = Buffer::new(&mut system, Metrics::new(font_size, font_size));
    buffer.set_size(&mut system, None, None);
    buffer.set_wrap(&mut system, Wrap::None);
    buffer.set_text(
        &mut system,
        text,
        &Attrs::new()
            .family(Family::Name(family_name(font)))
            .weight(Weight(font_weight)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut system, false);
    buffer
        .layout_runs()
        .next()
        .map(|run| line_from_layout_run(&system, run, font))
        .unwrap_or_else(|| fallback_line(text))
}

fn fallback_line(text: &str) -> TextLine {
    TextLine {
        text: text.to_owned(),
        runs: None,
    }
}

fn line_from_layout_run(system: &FontSystem, run: LayoutRun<'_>, base_font: SceneFont) -> TextLine {
    let glyph_range: Option<(usize, usize)> = run.glyphs.iter().fold(None, |range, glyph| {
        Some(match range {
            Some((start, end)) => (start.min(glyph.start), end.max(glyph.end)),
            None => (glyph.start, glyph.end),
        })
    });
    let Some((glyph_start, glyph_end)) = glyph_range else {
        return TextLine {
            text: String::new(),
            runs: Some(Vec::new()),
        };
    };
    let (line_start, line_end) = (glyph_start, glyph_end);
    let Some(raw_line) = run.text.get(line_start..line_end) else {
        return fallback_line(run.text);
    };
    let trimmed_line = raw_line.trim();
    if trimmed_line.is_empty() {
        return TextLine {
            text: String::new(),
            runs: Some(Vec::new()),
        };
    }
    let trim_start = line_start + raw_line.len() - raw_line.trim_start().len();
    let trim_end = trim_start + trimmed_line.len();

    let mut cluster_profiles = BTreeMap::<(usize, usize), Option<SceneFont>>::new();
    for glyph in run.glyphs {
        if glyph.start >= glyph.end
            || glyph.start < trim_start
            || glyph.end > trim_end
            || run.text.get(glyph.start..glyph.end).is_none()
        {
            continue;
        }
        let Some(face) = system.db().face(glyph.font_id) else {
            return fallback_line(trimmed_line);
        };
        let profile = [SceneFont::Sans, SceneFont::Serif, SceneFont::Mono]
            .into_iter()
            .find(|profile| {
                face.families
                    .iter()
                    .any(|(name, _)| name == family_name(*profile))
            })
            .unwrap_or(base_font);
        let selected = cluster_profiles
            .entry((glyph.start, glyph.end))
            .or_insert(Some(profile));
        if *selected != Some(profile) {
            *selected = None;
        }
    }
    if cluster_profiles.values().any(Option::is_none) {
        return fallback_line(trimmed_line);
    }

    let clusters = cluster_profiles
        .into_iter()
        .map(|((start, end), profile)| ClusterFont {
            start,
            end,
            profile: profile.unwrap_or(base_font),
        })
        .collect::<Vec<_>>();
    let Some(merged_clusters) = merge_overlapping_clusters(clusters) else {
        return fallback_line(trimmed_line);
    };

    let mut runs = Vec::<FontRun>::new();
    let mut position = trim_start;
    for cluster in merged_clusters {
        let start = cluster.start.max(trim_start);
        let end = cluster.end.min(trim_end);
        if start > position {
            append_run(
                &mut runs,
                run.text.get(position..start).unwrap_or_default(),
                base_font,
            );
        }
        if start < position || end < start {
            return fallback_line(trimmed_line);
        }
        append_run(
            &mut runs,
            run.text.get(start..end).unwrap_or_default(),
            cluster.profile,
        );
        position = end;
    }
    if position < trim_end {
        append_run(
            &mut runs,
            run.text.get(position..trim_end).unwrap_or_default(),
            base_font,
        );
    }

    let mut offset = 0;
    for pair in runs.windows(2) {
        offset += pair[0].text.len();
        if pair[0].profile != pair[1].profile && !safe_font_boundary(trimmed_line, offset) {
            return fallback_line(trimmed_line);
        }
    }
    TextLine {
        text: trimmed_line.to_owned(),
        runs: Some(runs),
    }
}

fn merge_overlapping_clusters(mut clusters: Vec<ClusterFont>) -> Option<Vec<ClusterFont>> {
    clusters.sort_by_key(|cluster| (cluster.start, cluster.end));
    let mut merged_clusters = Vec::<ClusterFont>::new();
    for cluster in clusters {
        if let Some(previous) = merged_clusters.last_mut()
            && cluster.start < previous.end
        {
            if previous.profile != cluster.profile {
                return None;
            }
            previous.end = previous.end.max(cluster.end);
        } else {
            merged_clusters.push(cluster);
        }
    }
    Some(merged_clusters)
}

fn append_run(runs: &mut Vec<FontRun>, text: &str, profile: SceneFont) {
    if text.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut()
        && previous.profile == profile
    {
        previous.text.push_str(text);
    } else {
        runs.push(FontRun {
            text: text.to_owned(),
            profile,
        });
    }
}

fn safe_font_boundary(text: &str, byte_offset: usize) -> bool {
    if byte_offset == 0 || byte_offset == text.len() {
        return true;
    }
    if !text
        .grapheme_indices(true)
        .any(|(start, _)| start == byte_offset)
    {
        return false;
    }
    let Some(previous_script) = text[..byte_offset]
        .chars()
        .rev()
        .map(|character| character.script())
        .find(|script| *script != Script::Inherited)
    else {
        return false;
    };
    let Some(next_script) = text[byte_offset..]
        .chars()
        .map(|character| character.script())
        .find(|script| *script != Script::Inherited)
    else {
        return false;
    };
    if matches!(previous_script, Script::Unknown | Script::Inherited)
        || matches!(next_script, Script::Unknown)
    {
        return false;
    }
    previous_script == Script::Common
        || next_script == Script::Common
        || previous_script != next_script
}

/// Return the bundled profiles actually selected when shaping non-ASCII text.
/// The renderer embeds these faces so SVG viewers can use the same fallbacks.
pub(super) fn font_profiles_for_text(font: SceneFont, text: &str, font_weight: u16) -> [bool; 3] {
    let mut used = [false; 3];
    if text.is_empty() || text.is_ascii() {
        return used;
    }
    let mut system = font_system()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut buffer = Buffer::new(&mut system, Metrics::new(16.0, 16.0));
    buffer.set_size(&mut system, None, None);
    buffer.set_text(
        &mut system,
        text,
        &Attrs::new()
            .family(Family::Name(family_name(font)))
            .weight(Weight(font_weight)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut system, false);
    for glyph in buffer.layout_runs().flat_map(|run| run.glyphs.iter()) {
        if glyph.glyph_id == 0 {
            continue;
        }
        if let Some(face) = system.db().face(glyph.font_id) {
            for (index, profile) in [SceneFont::Sans, SceneFont::Serif, SceneFont::Mono]
                .into_iter()
                .enumerate()
            {
                if face
                    .families
                    .iter()
                    .any(|(name, _)| name == family_name(profile))
                {
                    used[index] = true;
                }
            }
        }
    }
    used
}

/// Returns the horizontal room available to a node's text column.
pub fn node_text_width(
    bounds_width: f32,
    layout: SceneNodeLayout,
    has_icon: bool,
    content_gap: f32,
) -> f32 {
    let horizontal_icon_offset = layout == SceneNodeLayout::Horizontal && has_icon;
    (bounds_width
        - if horizontal_icon_offset {
            66.0 + content_gap
        } else {
            36.0
        })
    .max(1.0)
}

#[cfg(test)]
mod tests;
