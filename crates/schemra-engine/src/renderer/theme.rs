use schemra_scene::{SceneFont, SceneIconPack, ScenePalette, SceneTheme, SceneVariant};

pub(super) struct PaletteColors<'theme> {
    pub(super) canvas: &'theme str,
    pub(super) card: &'theme str,
    pub(super) edge: &'theme str,
    pub(super) text: &'theme str,
    pub(super) muted: &'theme str,
    pub(super) border: &'theme str,
    pub(super) icon_background: &'theme str,
    pub(super) icon_foreground: &'theme str,
    pub(super) solid_icon_background: &'theme str,
    pub(super) solid_icon_foreground: &'theme str,
    pub(super) shadow: &'theme str,
}

pub(super) struct VariantColors<'theme> {
    pub(super) fill: &'theme str,
    pub(super) stroke: &'theme str,
    pub(super) text: &'theme str,
}

fn palette_defaults(palette: ScenePalette) -> PaletteColors<'static> {
    match palette {
        ScenePalette::Paper => PaletteColors {
            canvas: "#ffffff",
            card: "#ffffff",
            edge: "#71717a",
            text: "#18181b",
            muted: "#52525b",
            border: "#e4e4e7",
            icon_background: "#f4f4f5",
            icon_foreground: "#52525b",
            solid_icon_background: "#52525b",
            solid_icon_foreground: "#ffffff",
            shadow: "#18181b",
        },
        ScenePalette::Midnight => PaletteColors {
            canvas: "#0f172a",
            card: "#1e293b",
            edge: "#94a3b8",
            text: "#f8fafc",
            muted: "#cbd5e1",
            border: "#475569",
            icon_background: "#334155",
            icon_foreground: "#cbd5e1",
            solid_icon_background: "#cbd5e1",
            solid_icon_foreground: "#0f172a",
            shadow: "#020617",
        },
        ScenePalette::Lagoon => PaletteColors {
            canvas: "#f0fdfa",
            card: "#ffffff",
            edge: "#0f766e",
            text: "#134e4a",
            muted: "#0f766e",
            border: "#99f6e4",
            icon_background: "#ccfbf1",
            icon_foreground: "#0f766e",
            solid_icon_background: "#0f766e",
            solid_icon_foreground: "#ecfdf5",
            shadow: "#134e4a",
        },
        ScenePalette::Sand => PaletteColors {
            canvas: "#fffcf5",
            card: "#ffffff",
            edge: "#a16207",
            text: "#422006",
            muted: "#854d0e",
            border: "#f5e6c8",
            icon_background: "#fef3c7",
            icon_foreground: "#854d0e",
            solid_icon_background: "#854d0e",
            solid_icon_foreground: "#fffbeb",
            shadow: "#422006",
        },
    }
}

pub(super) fn palette_colors(theme: &SceneTheme) -> PaletteColors<'_> {
    let defaults = palette_defaults(theme.palette);
    let colors = &theme.colors;
    PaletteColors {
        canvas: safe_color(colors.canvas.as_deref(), defaults.canvas),
        card: safe_color(colors.card.as_deref(), defaults.card),
        edge: safe_color(colors.edge.as_deref(), defaults.edge),
        text: safe_color(colors.text.as_deref(), defaults.text),
        muted: safe_color(colors.muted.as_deref(), defaults.muted),
        border: safe_color(colors.border.as_deref(), defaults.border),
        icon_background: safe_color(colors.icon_background.as_deref(), defaults.icon_background),
        icon_foreground: safe_color(colors.icon_foreground.as_deref(), defaults.icon_foreground),
        solid_icon_background: safe_color(
            colors.solid_icon_background.as_deref(),
            defaults.solid_icon_background,
        ),
        solid_icon_foreground: safe_color(
            colors.solid_icon_foreground.as_deref(),
            defaults.solid_icon_foreground,
        ),
        shadow: safe_color(colors.shadow.as_deref(), defaults.shadow),
    }
}

pub(super) fn variant_colors(theme: &SceneTheme, variant: SceneVariant) -> VariantColors<'_> {
    let defaults = variant_defaults(theme.palette, variant);
    let custom = theme.variants.iter().find(|item| item.variant == variant);
    VariantColors {
        fill: safe_color(custom.and_then(|item| item.fill.as_deref()), defaults.fill),
        stroke: safe_color(
            custom.and_then(|item| item.stroke.as_deref()),
            defaults.stroke,
        ),
        text: safe_color(custom.and_then(|item| item.text.as_deref()), defaults.text),
    }
}

pub(super) fn resolve_color<'theme>(
    value: Option<&'theme str>,
    fallback: &'theme str,
    theme: &'theme SceneTheme,
) -> &'theme str {
    let Some(value) = value else {
        return fallback;
    };
    let colors = palette_colors(theme);
    match value {
        "accent" => variant_colors(theme, SceneVariant::Accent).stroke,
        "success" => variant_colors(theme, SceneVariant::Success).stroke,
        "warning" => variant_colors(theme, SceneVariant::Warning).stroke,
        "danger" => variant_colors(theme, SceneVariant::Danger).stroke,
        "muted" | "subtle" => colors.muted,
        "text" => colors.text,
        "surface" => colors.card,
        "background" => colors.canvas,
        "border" => colors.border,
        "edge" => colors.edge,
        _ => safe_color(Some(value), fallback),
    }
}

pub(super) fn palette_name(theme: &SceneTheme) -> &str {
    theme.name.as_deref().unwrap_or(match theme.palette {
        ScenePalette::Paper => "paper",
        ScenePalette::Midnight => "midnight",
        ScenePalette::Lagoon => "lagoon",
        ScenePalette::Sand => "sand",
    })
}

pub(super) fn font_name(font: SceneFont) -> &'static str {
    match font {
        SceneFont::Sans => "sans",
        SceneFont::Serif => "serif",
        SceneFont::Mono => "mono",
    }
}

pub(super) fn icon_pack_name(icon_pack: SceneIconPack) -> &'static str {
    match icon_pack {
        SceneIconPack::Outline => "outline",
        SceneIconPack::Solid => "solid",
    }
}

fn variant_defaults(palette: ScenePalette, variant: SceneVariant) -> VariantColors<'static> {
    match (palette, variant) {
        (_, SceneVariant::Default) => VariantColors {
            fill: "#ffffff",
            stroke: "#e4e4e7",
            text: "#18181b",
        },
        (ScenePalette::Paper, SceneVariant::Muted) => VariantColors {
            fill: "#fafafa",
            stroke: "#d4d4d8",
            text: "#52525b",
        },
        (ScenePalette::Midnight, SceneVariant::Muted) => VariantColors {
            fill: "#111c2e",
            stroke: "#334155",
            text: "#cbd5e1",
        },
        (ScenePalette::Lagoon, SceneVariant::Muted) => VariantColors {
            fill: "#ccfbf1",
            stroke: "#99f6e4",
            text: "#0f766e",
        },
        (ScenePalette::Sand, SceneVariant::Muted) => VariantColors {
            fill: "#fef3c7",
            stroke: "#fcd34d",
            text: "#854d0e",
        },
        (ScenePalette::Paper, SceneVariant::Accent) => VariantColors {
            fill: "#eff6ff",
            stroke: "#2563eb",
            text: "#1d4ed8",
        },
        (ScenePalette::Midnight, SceneVariant::Accent) => VariantColors {
            fill: "#1e3a8a",
            stroke: "#60a5fa",
            text: "#dbeafe",
        },
        (ScenePalette::Lagoon, SceneVariant::Accent) => VariantColors {
            fill: "#ccfbf1",
            stroke: "#0f766e",
            text: "#134e4a",
        },
        (ScenePalette::Sand, SceneVariant::Accent) => VariantColors {
            fill: "#fef3c7",
            stroke: "#a16207",
            text: "#422006",
        },
        (_, SceneVariant::Success) => VariantColors {
            fill: "#ecfdf5",
            stroke: "#16a34a",
            text: "#166534",
        },
        (_, SceneVariant::Warning) => VariantColors {
            fill: "#fffbeb",
            stroke: "#f59e0b",
            text: "#92400e",
        },
        (_, SceneVariant::Danger) => VariantColors {
            fill: "#fef2f2",
            stroke: "#dc2626",
            text: "#991b1b",
        },
    }
}

fn safe_color<'theme>(value: Option<&'theme str>, fallback: &'theme str) -> &'theme str {
    value
        .filter(|value| {
            if let Some(hex) = value.strip_prefix('#') {
                return matches!(hex.len(), 3 | 4 | 6 | 8)
                    && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
            }
            !value.is_empty()
                && value.len() <= 32
                && value.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        .unwrap_or(fallback)
}
