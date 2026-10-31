//! Deterministic placement of visual IR nodes into renderer-neutral scenes.

use std::collections::{HashMap, HashSet, VecDeque};

use schemra_ir::{Direction, EdgeDirectionality, GroupLayout, VisualIr};
use schemra_renderer::{node_text_width, wrap_text_lines};
use schemra_scene::{
    FlowDirection, Rect, Scene, SceneEdge, SceneGroup, SceneNode, SceneNodeLayout, SceneNodeShape,
    SceneStyle, SceneTextAlign,
};

mod route_bounds;

const NODE_WIDTH: f32 = 252.0;
const NODE_GAP: f32 = 64.0;
const GROUP_NODE_GAP: f32 = 88.0;
const RANK_GAP: f32 = 112.0;
const CANVAS_PADDING: f32 = 48.0;
const GROUP_PADDING_X: f32 = 26.0;
const GROUP_PADDING_TOP: f32 = 42.0;
const GROUP_PADDING_BOTTOM: f32 = 26.0;

#[derive(Clone)]
struct LayoutBlock {
    members: Vec<usize>,
    group: Option<String>,
    columns: usize,
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    gap: f32,
    width: f32,
    height: f32,
}

/// Places visual nodes and groups into a scene while preserving source order
/// for ties. Link topology is copied into the scene; route geometry is added by
/// the routing stage.
pub fn layout_visual(visual: &VisualIr) -> Scene {
    let widths = vec![NODE_WIDTH; visual.nodes.len()];
    let heights = visual
        .nodes
        .iter()
        .map(|node| node_height(&node.label, node.detail.as_deref(), node.icon.is_some()))
        .collect::<Vec<_>>();
    let node_indices = visual
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.0.as_str(), index))
        .collect::<HashMap<_, _>>();
    let group_layouts = visual
        .groups
        .iter()
        .map(|group| (group.id.0.as_str(), group.layout))
        .collect::<HashMap<_, _>>();

    let mut flowing_groups = HashSet::new();
    for edge in &visual.edges {
        let (Some(from), Some(to)) = (
            node_indices.get(edge.from.0.as_str()),
            node_indices.get(edge.to.0.as_str()),
        ) else {
            continue;
        };
        if let (Some(group), Some(target_group)) = (
            visual.nodes[*from].group.as_ref(),
            visual.nodes[*to].group.as_ref(),
        ) && group == target_group
        {
            flowing_groups.insert(group.0.as_str());
        }
    }

    let (units, node_to_unit) =
        build_layout_units(visual, &widths, &heights, &group_layouts, &flowing_groups);
    let unit_edges = visual
        .edges
        .iter()
        .filter_map(|edge| {
            let from_node = *node_indices.get(edge.from.0.as_str())?;
            let to_node = *node_indices.get(edge.to.0.as_str())?;
            let from = node_to_unit[from_node];
            let to = node_to_unit[to_node];
            (from != to).then_some((from, to))
        })
        .collect::<Vec<_>>();
    let ranks = calculate_ranks(units.len(), &unit_edges);
    let max_rank = ranks.iter().copied().max().unwrap_or(0);
    let mut levels = vec![Vec::new(); max_rank + 1];
    for (unit, rank) in ranks.iter().copied().enumerate() {
        levels[rank].push(unit);
    }
    minimize_crossings(&mut levels, &ranks, &unit_edges);

    let blocks = levels
        .iter()
        .map(|level| {
            level
                .iter()
                .map(|unit| units[*unit].clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let horizontal = matches!(
        visual.direction,
        Direction::LeftToRight | Direction::RightToLeft
    );
    let reverse = matches!(
        visual.direction,
        Direction::RightToLeft | Direction::BottomToTop
    );
    let offsets = rank_offsets(&blocks, horizontal, reverse);
    let max_secondary = blocks
        .iter()
        .map(|level| secondary_extent(level, horizontal))
        .reduce(f32::max)
        .unwrap_or(0.0);
    let max_rank = blocks.len().saturating_sub(1);
    let terminal_level = if reverse { 0 } else { max_rank };
    let rank_extent = offsets.last().copied().unwrap_or(0.0)
        + primary_extent(&blocks[terminal_level], horizontal);

    let mut positioned_nodes = Vec::with_capacity(visual.nodes.len());
    let mut group_bounds = HashMap::<String, Rect>::new();
    for (rank, level) in blocks.iter().enumerate() {
        let display_rank = if reverse { max_rank - rank } else { rank };
        let level_secondary = secondary_extent(level, horizontal);
        let mut secondary = CANVAS_PADDING + (max_secondary - level_secondary) / 2.0;
        let primary = CANVAS_PADDING + offsets[display_rank];
        for (position, block) in level.iter().enumerate() {
            let mut row_offset = 0.0;
            for (row, row_height) in block.row_heights.iter().copied().enumerate() {
                let start = row * block.columns;
                let end = (start + block.columns).min(block.members.len());
                for slot in start..end {
                    let index = block.members[slot];
                    let column = slot % block.columns;
                    let cross_offset = block.column_widths[..column]
                        .iter()
                        .map(|width| width + block.gap)
                        .sum::<f32>();
                    let row_y = if horizontal { secondary } else { primary };
                    let x = if horizontal {
                        primary + cross_offset
                    } else {
                        secondary + cross_offset
                    };
                    let y = row_y + row_offset + (row_height - heights[index]) / 2.0;
                    let node = &visual.nodes[index];
                    let bounds = Rect {
                        x,
                        y,
                        width: widths[index],
                        height: heights[index],
                    };
                    if let Some(group) = node.group.as_ref() {
                        include_bounds(
                            group_bounds.entry(group.0.clone()).or_insert(bounds),
                            bounds,
                        );
                    }
                    positioned_nodes.push((
                        index,
                        SceneNode {
                            id: node.id.0.clone(),
                            label: node.label.clone(),
                            icon: node.icon.as_deref().map(|icon| {
                                icon.strip_prefix("lucide/").unwrap_or(icon).to_owned()
                            }),
                            detail: node.detail.clone(),
                            layout: SceneNodeLayout::Horizontal,
                            text_align: SceneTextAlign::Left,
                            shape: SceneNodeShape::Round,
                            style: SceneStyle::default(),
                            bounds,
                            group: node.group.as_ref().map(|group| group.0.clone()),
                        },
                    ));
                }
                row_offset += row_height + block.gap;
            }
            secondary += cross_extent(block, horizontal) + secondary_gap(level, position);
        }
    }
    positioned_nodes.sort_by_key(|(index, _)| *index);
    let mut nodes = positioned_nodes
        .into_iter()
        .map(|(_, node)| node)
        .collect::<Vec<_>>();

    let mut groups = visual
        .groups
        .iter()
        .filter_map(|group| {
            let mut bounds = *group_bounds.get(&group.id.0)?;
            bounds.x -= GROUP_PADDING_X;
            bounds.y -= GROUP_PADDING_TOP;
            bounds.width += GROUP_PADDING_X * 2.0;
            bounds.height += GROUP_PADDING_TOP + GROUP_PADDING_BOTTOM;
            Some(SceneGroup {
                name: group.id.0.clone(),
                title: group.title.clone(),
                bounds,
                style: SceneStyle::default(),
            })
        })
        .collect::<Vec<_>>();
    let mut edges = visual
        .edges
        .iter()
        .map(|edge| SceneEdge {
            id: None,
            source_id: None,
            reciprocal_edge_index: None,
            from: edge.from.0.clone(),
            to: edge.to.0.clone(),
            arrow_at_start: false,
            arrow_at_end: edge.directionality == EdgeDirectionality::Directed,
            label: None,
            style: SceneStyle::default(),
            group: None,
            path: Vec::new(),
            route_diagnostic: None,
        })
        .collect::<Vec<_>>();
    crate::routing::route_edges(&mut edges, &nodes, flow_direction(visual.direction));
    let (width, height) = route_bounds::fit_scene(&mut nodes, &mut groups, &mut edges, rank_extent);

    Scene {
        title: visual.title.clone(),
        direction: flow_direction(visual.direction),
        theme: Default::default(),
        width,
        height,
        groups,
        nodes,
        texts: Vec::new(),
        images: Vec::new(),
        icons: Vec::new(),
        edges,
    }
}

fn build_layout_units(
    visual: &VisualIr,
    widths: &[f32],
    heights: &[f32],
    group_layouts: &HashMap<&str, GroupLayout>,
    flowing_groups: &HashSet<&str>,
) -> (Vec<LayoutBlock>, Vec<usize>) {
    let mut units = Vec::new();
    let mut node_to_unit = vec![0; visual.nodes.len()];
    let mut grouped_units = HashMap::new();
    for (index, node) in visual.nodes.iter().enumerate() {
        let Some(group_id) = node.group.as_ref().map(|group| group.0.as_str()) else {
            node_to_unit[index] = units.len();
            units.push(single_layout_block(
                index,
                None,
                widths[index],
                heights[index],
            ));
            continue;
        };
        let layout = group_layouts.get(group_id).copied().unwrap_or_default();
        if layout == GroupLayout::Auto && flowing_groups.contains(group_id) {
            node_to_unit[index] = units.len();
            units.push(single_layout_block(
                index,
                Some(group_id),
                widths[index],
                heights[index],
            ));
            continue;
        }
        if let Some(unit) = grouped_units.get(group_id).copied() {
            node_to_unit[index] = unit;
            continue;
        }
        let members = visual
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(member, candidate)| {
                candidate
                    .group
                    .as_ref()
                    .is_some_and(|group| group.0 == group_id)
                    .then_some(member)
            })
            .collect::<Vec<_>>();
        let configured_columns = match layout {
            GroupLayout::Auto => None,
            GroupLayout::Grid { columns } => columns.map(|columns| columns as usize),
        };
        let unit = units.len();
        grouped_units.insert(group_id, unit);
        for member in &members {
            node_to_unit[*member] = unit;
        }
        units.push(grid_layout_block(
            members,
            group_id.to_owned(),
            configured_columns,
            widths,
            heights,
        ));
    }
    (units, node_to_unit)
}

fn single_layout_block(index: usize, group: Option<&str>, width: f32, height: f32) -> LayoutBlock {
    LayoutBlock {
        members: vec![index],
        group: group.map(str::to_owned),
        columns: 1,
        column_widths: vec![width],
        row_heights: vec![height],
        gap: NODE_GAP,
        width,
        height,
    }
}

fn grid_layout_block(
    members: Vec<usize>,
    group: String,
    configured_columns: Option<usize>,
    widths: &[f32],
    heights: &[f32],
) -> LayoutBlock {
    let member_count = members.len().max(1);
    let columns = configured_columns
        .unwrap_or_else(|| auto_grid_columns(member_count))
        .clamp(1, member_count);
    let row_count = member_count.div_ceil(columns);
    let mut row_heights = vec![0.0f32; row_count];
    let mut column_widths = vec![0.0f32; columns];
    for (slot, member) in members.iter().enumerate() {
        row_heights[slot / columns] = row_heights[slot / columns].max(heights[*member]);
        column_widths[slot % columns] = column_widths[slot % columns].max(widths[*member]);
    }
    let height = row_heights.iter().sum::<f32>() + (row_count.saturating_sub(1) as f32) * NODE_GAP;
    let width = column_widths.iter().sum::<f32>() + (columns.saturating_sub(1) as f32) * NODE_GAP;
    LayoutBlock {
        members,
        group: Some(group),
        columns,
        column_widths,
        row_heights,
        gap: NODE_GAP,
        width,
        height,
    }
}

fn auto_grid_columns(member_count: usize) -> usize {
    let mut columns = 1;
    while columns < member_count.div_ceil(columns) {
        columns += 1;
    }
    columns
}

fn calculate_ranks(unit_count: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    let mut ranks = vec![0; unit_count];
    let mut indegree = vec![0usize; unit_count];
    let mut adjacency = vec![Vec::new(); unit_count];
    let mut seen_pairs = HashSet::new();
    for (from, to) in edges {
        if from != to && seen_pairs.insert((from.min(to), from.max(to))) {
            adjacency[*from].push(*to);
            indegree[*to] += 1;
        }
    }
    let mut queue = VecDeque::new();
    for (index, count) in indegree.iter().enumerate() {
        if *count == 0 {
            queue.push_back(index);
        }
    }
    while let Some(source) = queue.pop_front() {
        for target in &adjacency[source] {
            ranks[*target] = ranks[*target].max(ranks[source] + 1);
            indegree[*target] -= 1;
            if indegree[*target] == 0 {
                queue.push_back(*target);
            }
        }
    }
    ranks
}

fn minimize_crossings(levels: &mut [Vec<usize>], ranks: &[usize], edges: &[(usize, usize)]) {
    if levels.len() < 2 {
        return;
    }
    let mut predecessors = vec![Vec::new(); ranks.len()];
    let mut successors = vec![Vec::new(); ranks.len()];
    for (from, to) in edges {
        if ranks[*from] == ranks[*to] {
            continue;
        }
        let (earlier, later) = if ranks[*from] < ranks[*to] {
            (*from, *to)
        } else {
            (*to, *from)
        };
        successors[earlier].push(later);
        predecessors[later].push(earlier);
    }
    for neighbors in predecessors.iter_mut().chain(successors.iter_mut()) {
        neighbors.sort_unstable();
        neighbors.dedup();
    }
    for _ in 0..4 {
        for rank in 1..levels.len() {
            let positions = layer_positions(levels, ranks.len());
            reorder_layer(&mut levels[rank], &predecessors, &positions);
        }
        for rank in (0..levels.len() - 1).rev() {
            let positions = layer_positions(levels, ranks.len());
            reorder_layer(&mut levels[rank], &successors, &positions);
        }
    }
}

fn layer_positions(levels: &[Vec<usize>], unit_count: usize) -> Vec<usize> {
    let mut positions = vec![0; unit_count];
    for level in levels {
        for (position, unit) in level.iter().copied().enumerate() {
            positions[unit] = position;
        }
    }
    positions
}

fn reorder_layer(level: &mut [usize], neighbors: &[Vec<usize>], positions: &[usize]) {
    let originals = level
        .iter()
        .copied()
        .enumerate()
        .map(|(position, unit)| (unit, position))
        .collect::<HashMap<_, _>>();
    let barycenters = level
        .iter()
        .copied()
        .map(|unit| {
            let adjacent = &neighbors[unit];
            let average = (!adjacent.is_empty()).then(|| {
                adjacent
                    .iter()
                    .map(|neighbor| positions[*neighbor] as f32)
                    .sum::<f32>()
                    / adjacent.len() as f32
            });
            (unit, average)
        })
        .collect::<HashMap<_, _>>();
    level.sort_by(|left, right| {
        barycenters[left]
            .unwrap_or(originals[left] as f32)
            .total_cmp(&barycenters[right].unwrap_or(originals[right] as f32))
            .then_with(|| originals[left].cmp(&originals[right]))
    });
}

fn rank_offsets(levels: &[Vec<LayoutBlock>], horizontal: bool, reverse: bool) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(levels.len());
    let mut offset = 0.0;
    for display_rank in 0..levels.len() {
        let rank = if reverse {
            levels.len() - display_rank - 1
        } else {
            display_rank
        };
        offsets.push(offset);
        offset += primary_extent(&levels[rank], horizontal) + RANK_GAP;
    }
    offsets
}

fn primary_extent(level: &[LayoutBlock], horizontal: bool) -> f32 {
    level
        .iter()
        .map(|block| {
            if horizontal {
                block.width
            } else {
                block.height
            }
        })
        .reduce(f32::max)
        .unwrap_or(0.0)
}

fn cross_extent(block: &LayoutBlock, horizontal: bool) -> f32 {
    if horizontal {
        block.height
    } else {
        block.width
    }
}

fn secondary_extent(level: &[LayoutBlock], horizontal: bool) -> f32 {
    level
        .iter()
        .enumerate()
        .map(|(position, block)| cross_extent(block, horizontal) + secondary_gap(level, position))
        .sum()
}

fn secondary_gap(level: &[LayoutBlock], position: usize) -> f32 {
    let Some(next) = level.get(position + 1) else {
        return 0.0;
    };
    let current_group = level[position].group.as_deref();
    let next_group = next.group.as_deref();
    if current_group == next_group {
        current_group.map_or(NODE_GAP, |_| level[position].gap)
    } else if current_group.is_some() || next_group.is_some() {
        GROUP_NODE_GAP
    } else {
        NODE_GAP
    }
}

fn node_height(label: &str, detail: Option<&str>, has_icon: bool) -> f32 {
    let text_width = node_text_width(NODE_WIDTH, SceneNodeLayout::Horizontal, has_icon, 12.0);
    let title_lines = wrap_text_lines(schemra_scene::SceneFont::Sans, label, text_width, 17.0, 650)
        .len()
        .max(1);
    let detail_lines = detail
        .map(|text| {
            wrap_text_lines(schemra_scene::SceneFont::Sans, text, text_width, 14.0, 450).len()
        })
        .unwrap_or(0);
    let text_height = title_lines as f32 * 22.0
        + if detail_lines > 0 {
            4.0 + detail_lines as f32 * 18.0
        } else {
            0.0
        };
    text_height.max(if has_icon { 30.0 } else { 0.0 }) + 32.0
}

fn include_bounds(target: &mut Rect, bounds: Rect) {
    let right = (target.x + target.width).max(bounds.x + bounds.width);
    let bottom = (target.y + target.height).max(bounds.y + bounds.height);
    target.x = target.x.min(bounds.x);
    target.y = target.y.min(bounds.y);
    target.width = right - target.x;
    target.height = bottom - target.y;
}

fn flow_direction(direction: Direction) -> FlowDirection {
    match direction {
        Direction::LeftToRight => FlowDirection::LeftToRight,
        Direction::RightToLeft => FlowDirection::RightToLeft,
        Direction::TopToBottom => FlowDirection::TopToBottom,
        Direction::BottomToTop => FlowDirection::BottomToTop,
    }
}
