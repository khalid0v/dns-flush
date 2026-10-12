//! Microsoft Fluent-inspired colors and spacing for the desktop interface.
//!
//! Pure styling: no I/O, no OS access. Call `apply` at startup and when the
//! user changes the appearance.

use egui::{
    Color32, Context, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Visuals,
};

struct Palette {
    window: Color32,
    sidebar: Color32,
    field: Color32,
    control: Color32,
    control_hover: Color32,
    control_press: Color32,
    border: Color32,
    separator: Color32,
    text: Color32,
    text_secondary: Color32,
    accent: Color32,
    warning: Color32,
    success: Color32,
    destructive: Color32,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            window: Color32::from_rgb(32, 32, 32),
            sidebar: Color32::from_rgb(40, 40, 40),
            field: Color32::from_rgb(28, 28, 28),
            control: Color32::from_rgb(48, 48, 48),
            control_hover: Color32::from_rgb(58, 58, 58),
            control_press: Color32::from_rgb(70, 70, 70),
            border: Color32::from_rgb(92, 92, 92),
            separator: Color32::from_rgb(64, 64, 64),
            text: Color32::from_rgb(245, 245, 245),
            text_secondary: Color32::from_rgb(200, 200, 200),
            accent: Color32::from_rgb(96, 173, 230),
            warning: Color32::from_rgb(255, 185, 100),
            success: Color32::from_rgb(110, 210, 140),
            destructive: Color32::from_rgb(215, 64, 64),
        }
    } else {
        Palette {
            window: Color32::from_rgb(245, 247, 250),
            sidebar: Color32::from_rgb(255, 255, 255),
            field: Color32::from_rgb(255, 255, 255),
            control: Color32::from_rgb(255, 255, 255),
            control_hover: Color32::from_rgb(238, 244, 249),
            control_press: Color32::from_rgb(222, 235, 246),
            border: Color32::from_rgb(138, 146, 154),
            separator: Color32::from_rgb(220, 225, 230),
            text: Color32::from_rgb(36, 36, 36),
            text_secondary: Color32::from_rgb(82, 82, 82),
            accent: Color32::from_rgb(0, 90, 158),
            warning: Color32::from_rgb(138, 75, 0),
            success: Color32::from_rgb(15, 105, 55),
            destructive: Color32::from_rgb(177, 14, 28),
        }
    }
}

pub fn sidebar_fill(dark: bool) -> Color32 {
    palette(dark).sidebar
}

pub fn text(dark: bool) -> Color32 {
    palette(dark).text
}

pub fn secondary_text(dark: bool) -> Color32 {
    palette(dark).text_secondary
}

pub fn accent(dark: bool) -> Color32 {
    palette(dark).accent
}

pub fn warning(dark: bool) -> Color32 {
    palette(dark).warning
}

pub fn success(dark: bool) -> Color32 {
    palette(dark).success
}

pub fn destructive(dark: bool) -> Color32 {
    palette(dark).destructive
}

pub fn apply(ctx: &Context, dark: bool) {
    let p = palette(dark);
    let mut visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = p.window;
    visuals.window_fill = p.window;
    visuals.extreme_bg_color = p.field;
    visuals.faint_bg_color = p.sidebar;
    visuals.code_bg_color = p.field;
    visuals.hyperlink_color = p.accent;
    visuals.warn_fg_color = p.warning;
    visuals.error_fg_color = p.destructive;
    visuals.selection.bg_fill = p.accent;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

    let radius = CornerRadius::same(6);
    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = p.window;
    widgets.noninteractive.weak_bg_fill = p.window;
    widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.separator);
    widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    widgets.noninteractive.corner_radius = radius;

    widgets.inactive.bg_fill = p.control;
    widgets.inactive.weak_bg_fill = p.control;
    widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    widgets.inactive.fg_stroke = Stroke::new(1.0, p.text);
    widgets.inactive.corner_radius = radius;

    widgets.hovered.bg_fill = p.control_hover;
    widgets.hovered.weak_bg_fill = p.control_hover;
    widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent);
    widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    widgets.hovered.corner_radius = radius;
    widgets.hovered.expansion = 0.0;

    widgets.active.bg_fill = p.control_press;
    widgets.active.weak_bg_fill = p.control_press;
    widgets.active.bg_stroke = Stroke::new(2.0, p.accent);
    widgets.active.fg_stroke = Stroke::new(1.0, p.text);
    widgets.active.corner_radius = radius;
    widgets.active.expansion = 0.0;

    widgets.open.bg_fill = p.control_hover;
    widgets.open.weak_bg_fill = p.control_hover;
    widgets.open.bg_stroke = Stroke::new(1.0, p.border);
    widgets.open.fg_stroke = Stroke::new(1.0, p.text);
    widgets.open.corner_radius = radius;

    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.menu_corner_radius = CornerRadius::same(8);
    visuals.window_stroke = Stroke::new(1.0, p.separator);
    let shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 110 } else { 35 }),
    };
    visuals.window_shadow = shadow;
    visuals.popup_shadow = shadow;
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        use FontFamily::{Monospace, Proportional};
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, Proportional)),
            (TextStyle::Body, FontId::new(16.0, Proportional)),
            (TextStyle::Button, FontId::new(16.0, Proportional)),
            (TextStyle::Small, FontId::new(14.0, Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, Monospace)),
        ]
        .into();

        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 9.0);
        style.spacing.interact_size.y = 38.0;
        style.spacing.window_margin = Margin::same(20);
        style.spacing.menu_margin = Margin::same(8);
    });
}
