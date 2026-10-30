//! SVG paths, direction cues, and labels for routed links.

use std::fmt::Write as _;

use schemra_scene::{CubicBezier, Point, Scene, SceneEdge, SceneTheme, SceneVariant};

use super::{font_family, number, text, theme, write_xml};

mod geometry;

pub(super) fn write_paths(svg: &mut String, scene: &Scene, default_color: &str) {
    for (index, edge) in scene.edges.iter().enumerate() {
        if edge
            .reciprocal_edge_index
            .is_some_and(|peer_index| peer_index < index)
        {
            continue;
        }
        let reciprocal = edge
            .reciprocal_edge_index
            .filter(|peer_index| *peer_index > index)
            .and_then(|peer_index| scene.edges.get(peer_index).map(|peer| (peer_index, peer)))
            .filter(|(_, peer)| peer.reciprocal_edge_index == Some(index));
        let color = edge_color(edge, default_color, &scene.theme);
        let arrow_at_start =
            edge.arrow_at_start || reciprocal.is_some_and(|(_, peer)| peer.arrow_at_end);
        let arrow_at_end =
            edge.arrow_at_end || reciprocal.is_some_and(|(_, peer)| peer.arrow_at_start);
        let bidirectional = arrow_at_start && arrow_at_end;
        let mut cues = Vec::with_capacity(usize::from(arrow_at_start) + usize::from(arrow_at_end));
        if edge.arrow_at_start || reciprocal.is_some_and(|(_, peer)| peer.arrow_at_end) {
            cues.extend(geometry::route_direction_cue(
                &edge.path,
                true,
                bidirectional,
            ));
        }
        if edge.arrow_at_end || reciprocal.is_some_and(|(_, peer)| peer.arrow_at_start) {
            cues.extend(geometry::route_direction_cue(
                &edge.path,
                false,
                bidirectional,
            ));
        }
        write_edge_path(svg, index, edge, reciprocal, color, &cues);
    }
}

pub(super) fn write_labels(svg: &mut String, scene: &Scene, canvas: &str, fallback: &str) {
    for (index, edge) in scene.edges.iter().enumerate() {
        let Some(label) = edge.label.as_deref() else {
            continue;
        };
        if label.trim().is_empty() || edge.path.is_empty() {
            continue;
        }

        let baseline = geometry::route_label_baseline(&edge.path, label, &scene.nodes);
        let font_profile = edge.style.font.unwrap_or(scene.theme.font);
        let weight = edge.style.weight.unwrap_or(550);
        let line = text::unwrapped_text_line(font_profile, label, 12.0, weight);
        let semantic = theme::variant_colors(&scene.theme, edge.style.variant);
        let default_color = if edge.style.variant == SceneVariant::Default {
            fallback
        } else {
            semantic.text
        };
        write!(
            svg,
            "<text data-edge-index=\"{index}\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"12\" font-weight=\"{weight}\" fill=\"",
            number(baseline.x),
            number(baseline.y),
            edge.style
                .font
                .map(font_family)
                .unwrap_or_else(|| font_family(scene.theme.font)),
        )
        .expect("writing to a String cannot fail");
        write_xml(
            svg,
            theme::resolve_color(
                edge.style.text_color.as_deref(),
                default_color,
                &scene.theme,
            ),
        );
        write!(
            svg,
            "\" stroke=\"{canvas}\" stroke-width=\"5\" stroke-linejoin=\"round\" paint-order=\"stroke\""
        )
        .expect("writing to a String cannot fail");
        if let Some(opacity) = edge.style.opacity {
            write!(svg, " opacity=\"{}\"", number(opacity))
                .expect("writing to a String cannot fail");
        }
        svg.push('>');
        super::nodes::write_text_runs(svg, &line, font_profile);
        svg.push_str("</text>");
    }
}

fn edge_color<'edge>(
    edge: &'edge SceneEdge,
    fallback: &'edge str,
    theme_data: &'edge SceneTheme,
) -> &'edge str {
    let semantic = theme::variant_colors(theme_data, edge.style.variant);
    let default = if edge.style.variant == SceneVariant::Default {
        fallback
    } else {
        semantic.stroke
    };
    theme::resolve_color(edge.style.stroke.as_deref(), default, theme_data)
}

fn write_edge_path(
    svg: &mut String,
    index: usize,
    edge: &SceneEdge,
    reciprocal: Option<(usize, &SceneEdge)>,
    color: &str,
    cues: &[(Point, Point)],
) {
    let Some(mut path) = svg_path_data(&edge.path) else {
        return;
    };
    append_direction_cues(&mut path, cues);
    let edge_indices =
        reciprocal.map_or_else(|| index.to_string(), |(peer, _)| format!("{index},{peer}"));
    let edge_ids = std::iter::once(edge.id.as_deref())
        .chain(reciprocal.map(|(_, peer)| peer.id.as_deref()))
        .flatten()
        .collect::<Vec<_>>()
        .join(",");
    let source_ids = std::iter::once(edge.source_id.as_deref())
        .chain(reciprocal.map(|(_, peer)| peer.source_id.as_deref()))
        .flatten()
        .collect::<Vec<_>>()
        .join(",");

    write!(
        svg,
        "<path class=\"schemra-edge-path\" d=\"{path}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2.4\" stroke-linecap=\"round\" stroke-linejoin=\"round\" data-edge-path=\"true\" data-edge-index=\"{index}\" data-edge-indices=\"{edge_indices}\" data-arrow-start=\"{}\" data-arrow-end=\"{}\" data-from=\"",
        edge.arrow_at_start || reciprocal.is_some_and(|(_, peer)| peer.arrow_at_end),
        edge.arrow_at_end || reciprocal.is_some_and(|(_, peer)| peer.arrow_at_start),
    )
    .expect("writing to a String cannot fail");
    write_xml(svg, &edge.from);
    svg.push_str("\" data-to=\"");
    write_xml(svg, &edge.to);
    svg.push('"');
    if let Some(id) = edge.id.as_deref() {
        svg.push_str(" data-edge-id=\"");
        write_xml(svg, id);
        svg.push('"');
    }
    if !edge_ids.is_empty() {
        svg.push_str(" data-edge-ids=\"");
        write_xml(svg, &edge_ids);
        svg.push('"');
    }
    if !source_ids.is_empty() {
        svg.push_str(" data-edge-source-ids=\"");
        write_xml(svg, &source_ids);
        svg.push('"');
    }
    if let Some(opacity) = edge.style.opacity {
        write!(svg, " opacity=\"{}\"", number(opacity)).expect("writing to a String cannot fail");
    }
    svg.push_str("/>");
}

const DIRECTION_CUE_HALF_LENGTH: f32 = 4.0;
const DIRECTION_CUE_HALF_WIDTH: f32 = 3.5;

fn append_direction_cues(path: &mut String, cues: &[(Point, Point)]) {
    for (center, tangent) in cues {
        let normal = Point {
            x: -tangent.y,
            y: tangent.x,
        };
        let tip = offset(*center, *tangent, DIRECTION_CUE_HALF_LENGTH);
        let back = offset(*center, *tangent, -DIRECTION_CUE_HALF_LENGTH);
        let left = offset(back, normal, DIRECTION_CUE_HALF_WIDTH);
        let right = offset(back, normal, -DIRECTION_CUE_HALF_WIDTH);
        write!(
            path,
            " M {} {} L {} {} L {} {}",
            number(left.x),
            number(left.y),
            number(tip.x),
            number(tip.y),
            number(right.x),
            number(right.y),
        )
        .expect("writing to a String cannot fail");
    }
}

fn offset(point: Point, direction: Point, distance: f32) -> Point {
    Point {
        x: point.x + direction.x * distance,
        y: point.y + direction.y * distance,
    }
}

fn svg_path_data(segments: &[CubicBezier]) -> Option<String> {
    let mut data = String::new();
    let mut previous_end = None;
    for segment in segments.iter().filter(|segment| !is_degenerate(**segment)) {
        if previous_end.is_none_or(|end: Point| {
            (end.x - segment.start.x).hypot(end.y - segment.start.y) > 0.001
        }) {
            write!(
                data,
                "{}M {} {}",
                if data.is_empty() { "" } else { " " },
                number(segment.start.x),
                number(segment.start.y)
            )
            .expect("writing to a String cannot fail");
        }
        write!(
            data,
            " C {} {}, {} {}, {} {}",
            number(segment.control1.x),
            number(segment.control1.y),
            number(segment.control2.x),
            number(segment.control2.y),
            number(segment.end.x),
            number(segment.end.y),
        )
        .expect("writing to a String cannot fail");
        previous_end = Some(segment.end);
    }
    (!data.is_empty()).then_some(data)
}

fn is_degenerate(segment: CubicBezier) -> bool {
    segment.start == segment.control1
        && segment.start == segment.control2
        && segment.start == segment.end
}
