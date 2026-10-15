use egui::{Color32, Context, Rounding, Stroke, Visuals};

pub struct JetBrainsDark;

impl JetBrainsDark {
    pub const BG: Color32 = Color32::from_rgb(30, 31, 34);
    pub const PANEL: Color32 = Color32::from_rgb(43, 45, 48);
    pub const EDITOR: Color32 = Color32::from_rgb(25, 26, 28);
    pub const BORDER: Color32 = Color32::from_rgb(60, 63, 68);
    pub const TEXT: Color32 = Color32::from_rgb(188, 191, 196);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(120, 123, 128);
    pub const ACCENT: Color32 = Color32::from_rgb(53, 116, 240);
    pub const ACCENT_DIM: Color32 = Color32::from_rgb(45, 85, 165);
    pub const HEADER: Color32 = Color32::from_rgb(50, 52, 56);
    pub const ROW_ALT: Color32 = Color32::from_rgb(35, 36, 40);
    pub const TAB_ACTIVE: Color32 = Color32::from_rgb(50, 52, 56);
    pub const TAB_INACTIVE: Color32 = Color32::from_rgb(37, 38, 42);
    pub const SUCCESS: Color32 = Color32::from_rgb(75, 155, 95);
}

pub fn apply(ctx: &Context) {
    let mut v = Visuals::dark();
    v.dark_mode = true;
    v.override_text_color = Some(JetBrainsDark::TEXT);
    v.window_fill = JetBrainsDark::PANEL;
    v.panel_fill = JetBrainsDark::PANEL;
    v.extreme_bg_color = JetBrainsDark::BG;
    v.faint_bg_color = JetBrainsDark::EDITOR;
    v.code_bg_color = JetBrainsDark::EDITOR;
    v.window_stroke = Stroke::new(1.0, JetBrainsDark::BORDER);
    v.widgets.noninteractive.bg_fill = JetBrainsDark::PANEL;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, JetBrainsDark::TEXT_DIM);
    v.widgets.inactive.bg_fill = JetBrainsDark::TAB_INACTIVE;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, JetBrainsDark::TEXT);
    v.widgets.hovered.bg_fill = Color32::from_rgb(55, 58, 63);
    v.widgets.active.bg_fill = JetBrainsDark::ACCENT_DIM;
    v.widgets.open.bg_fill = JetBrainsDark::TAB_ACTIVE;
    v.selection.bg_fill = JetBrainsDark::ACCENT;
    v.selection.stroke = Stroke::new(1.0, JetBrainsDark::ACCENT);
    v.hyperlink_color = JetBrainsDark::ACCENT;
    v.warn_fg_color = Color32::from_rgb(230, 180, 80);
    v.error_fg_color = Color32::from_rgb(230, 100, 100);
    let r = Rounding::same(4.0);
    v.widgets.noninteractive.rounding = r;
    v.widgets.inactive.rounding = r;
    v.widgets.active.rounding = r;
    ctx.set_visuals(v);
}
