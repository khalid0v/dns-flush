//! Apple HIG-inspired (macOS-style) visuals for egui.
//!
//! Pure styling: no I/O, no OS access. Call `apply` once at startup and again
//! whenever the user switches theme or light/dark mode.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Visuals,
};

/// Colors follow the macOS system palette (light / dark).
struct Palette {
    window: Color32,  // main content background
    sidebar: Color32, // left navigation background
    field: Color32,   // text inputs, log area
    control: Color32, // buttons at rest
    control_hover: Color32,
    control_press: Color32,
    border: Color32,    // control outline
    separator: Color32, // hairlines between panels
    text: Color32,      // primary label
    text_secondary: Color32,
    accent: Color32,  // system blue
    warning: Color32, // system orange
    success: Color32, // system green
    destructive: Color32,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            window: Color32::from_rgb(38, 38, 40),
            sidebar: Color32::from_rgb(30, 30, 32),
            field: Color32::from_rgb(28, 28, 30),
            control: Color32::from_rgb(58, 58, 60),
            control_hover: Color32::from_rgb(72, 72, 74),
            control_press: Color32::from_rgb(88, 88, 90),
            border: Color32::from_rgb(72, 72, 74),
            separator: Color32::from_rgb(54, 54, 56),
            text: Color32::from_rgb(235, 235, 240),
            text_secondary: Color32::from_rgb(152, 152, 157),
            accent: Color32::from_rgb(10, 132, 255),
            warning: Color32::from_rgb(255, 159, 10),
            success: Color32::from_rgb(48, 209, 88),
            destructive: Color32::from_rgb(255, 69, 58),
        }
    } else {
        Palette {
            window: Color32::from_rgb(255, 255, 255),
            sidebar: Color32::from_rgb(242, 242, 245),
            field: Color32::from_rgb(255, 255, 255),
            control: Color32::from_rgb(255, 255, 255),
            control_hover: Color32::from_rgb(245, 245, 247),
            control_press: Color32::from_rgb(228, 228, 232),
            border: Color32::from_rgb(208, 208, 213),
            separator: Color32::from_rgb(229, 229, 234),
            text: Color32::from_rgb(28, 28, 30),
            text_secondary: Color32::from_rgb(110, 110, 115),
            accent: Color32::from_rgb(0, 122, 255),
            warning: Color32::from_rgb(255, 149, 0),
            success: Color32::from_rgb(52, 199, 89),
            destructive: Color32::from_rgb(255, 59, 48),
        }
    }
}

/// Background for the left navigation panel (use with `Frame::new().fill(..)`).
pub fn sidebar_fill(dark: bool) -> Color32 {
    palette(dark).sidebar
}

/// Primary label color.
pub fn text(dark: bool) -> Color32 {
    palette(dark).text
}

/// Muted color for captions and hints.
pub fn secondary_text(dark: bool) -> Color32 {
    palette(dark).text_secondary
}

/// System accent (blue) used for highlights and the active control outline.
pub fn accent(dark: bool) -> Color32 {
    palette(dark).accent
}

/// System orange, for warnings.
pub fn warning(dark: bool) -> Color32 {
    palette(dark).warning
}

/// System green, for success.
pub fn success(dark: bool) -> Color32 {
    palette(dark).success
}

/// Red used for destructive actions (e.g. the reset-adapter confirm button).
pub fn destructive(dark: bool) -> Color32 {
    palette(dark).destructive
}

/// Apply the Apple-style theme to the whole app.
pub fn apply(ctx: &Context, dark: bool) {
    let p = palette(dark);

    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    v.panel_fill = p.window;
    v.window_fill = p.window;
    v.extreme_bg_color = p.field; // TextEdit / log background
    v.faint_bg_color = p.sidebar; // striped rows
    v.code_bg_color = p.field;
    v.hyperlink_color = p.accent;
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.destructive;

    v.selection.bg_fill = p.accent;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    let radius = CornerRadius::same(6);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.window;
    w.noninteractive.weak_bg_fill = p.window;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.separator);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = radius;

    w.inactive.bg_fill = p.control;
    w.inactive.weak_bg_fill = p.control;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = p.control_hover;
    w.hovered.weak_bg_fill = p.control_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, p.border);
    w.hovered.fg_stroke = Stroke::new(1.0, p.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.control_press;
    w.active.weak_bg_fill = p.control_press;
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(1.0, p.text);
    w.active.corner_radius = radius;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.control_hover;
    w.open.weak_bg_fill = p.control_hover;
    w.open.bg_stroke = Stroke::new(1.0, p.border);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = radius;

    // Floating surfaces: larger radius, soft shadow, hairline border.
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.window_stroke = Stroke::new(1.0, p.separator);
    let shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 110 } else { 45 }),
    };
    v.window_shadow = shadow;
    v.popup_shadow = shadow;

    ctx.set_visuals(v);

    // Typography and spacing: 13 pt body like macOS, roomier controls.
    ctx.all_styles_mut(|style| {
        use FontFamily::{Monospace, Proportional};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(20.0, Proportional)),
            (TextStyle::Body, FontId::new(13.0, Proportional)),
            (TextStyle::Button, FontId::new(13.0, Proportional)),
            (TextStyle::Small, FontId::new(11.0, Proportional)),
            (TextStyle::Monospace, FontId::new(12.5, Monospace)),
        ]
        .into();

        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 5.0);
        style.spacing.interact_size.y = 24.0;
        style.spacing.window_margin = Margin::same(16);
        style.spacing.menu_margin = Margin::same(6);
    });
}
