//! ChronoCraft L3 Native Desktop Studio (`eframe` / `egui` 0.37).

use crate::subapps;
use chronocraft_core::{
    default_db_path, ChronoEngine, Command, ModelPreset, MonteCarloResult, ScenarioDiff,
    SensitivityResult, TimeSeries, APP_VERSION,
};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

const HELP_MD: &str = include_str!("../../../HELP.md");
const ATTRIBUTION_MD: &str = include_str!("../../../ATTRIBUTION.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLocale {
    En,
    De,
}

impl AppLocale {
    pub fn label(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::De => "Deutsch",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Canvas,
    Sliders,
    FanChart,
    Runs,
    Chat,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpTab {
    GetStarted,
    CanvasStocks,
    SlidersParams,
    BandsCompare,
    CliMcp,
    TechnicalInfo,
}

pub struct ChronoCraftApp {
    pub engine: ChronoEngine,
    pub db_path: PathBuf,
    pub page: Page,
    pub subpage: usize,
    pub locale: AppLocale,
    pub dark_mode: bool,
    pub zoom_factor: f32,
    pub status_line: String,
    // Canvas builder state
    pub new_stock_id: String,
    pub new_stock_label: String,
    pub new_stock_initial: f64,
    pub new_flow_id: String,
    pub new_flow_label: String,
    pub new_flow_from: String,
    pub new_flow_to: String,
    pub new_flow_expr: String,
    pub link_from: String,
    pub link_to: String,
    // Sliders & solver settings
    pub pending_params: HashMap<String, f64>,
    pub sim_steps: usize,
    pub sim_dt: f64,
    pub mc_runs: usize,
    pub mc_seed: u64,
    pub mc_std: f64,
    // Results viewers
    pub last_trace: Option<TimeSeries>,
    pub last_mc: Option<MonteCarloResult>,
    pub fan_stock: String,
    pub scenario_name: String,
    pub cmp_base: String,
    pub cmp_var: String,
    pub last_list: Vec<String>,
    pub last_diff: Option<ScenarioDiff>,
    pub sweep_param: String,
    pub sweep_target: String,
    pub sweep_from: f64,
    pub sweep_to: f64,
    pub sweep_steps: usize,
    pub last_sweep: Option<SensitivityResult>,
    // Assistant & modals
    pub chat_input: String,
    pub chat_history: Vec<(String, String)>,
    pub help_tab: HelpTab,
    pub show_palette: bool,
    pub palette_query: String,
    pub show_shortcuts: bool,
    pub show_licenses: bool,
    pub show_about: bool,
}

impl Default for ChronoCraftApp {
    fn default() -> Self {
        Self::with_db_path(default_db_path())
    }
}

impl ChronoCraftApp {
    pub fn with_db_path(db_path: PathBuf) -> Self {
        let engine = ChronoEngine::open_or_seed(&db_path).unwrap_or_default();
        let pending_params = engine
            .params
            .iter()
            .map(|p| (p.name.clone(), p.value))
            .collect();
        Self {
            status_line: format!(
                "ChronoCraft v{APP_VERSION} ready -- scenario: {}",
                engine.active_scenario
            ),
            fan_stock: engine
                .stocks
                .first()
                .map(|s| s.id.clone())
                .unwrap_or_default(),
            engine,
            db_path,
            page: Page::Canvas,
            subpage: 0,
            locale: AppLocale::En,
            dark_mode: true,
            zoom_factor: 1.0,
            new_stock_id: "stock_new".into(),
            new_stock_label: "New Stock".into(),
            new_stock_initial: 100.0,
            new_flow_id: "flow_new".into(),
            new_flow_label: "New Flow".into(),
            new_flow_from: String::new(),
            new_flow_to: String::new(),
            new_flow_expr: "0.1 * pop".into(),
            link_from: String::new(),
            link_to: String::new(),
            pending_params,
            sim_steps: 200,
            sim_dt: 0.05,
            mc_runs: 500,
            mc_seed: 42,
            mc_std: 0.05,
            last_trace: None,
            last_mc: None,
            scenario_name: "baseline".into(),
            cmp_base: "baseline".into(),
            cmp_var: "challenge".into(),
            last_list: Vec::new(),
            last_diff: None,
            sweep_param: "growth".into(),
            sweep_target: "pop".into(),
            sweep_from: 0.05,
            sweep_to: 0.2,
            sweep_steps: 5,
            last_sweep: None,
            chat_input: String::new(),
            chat_history: vec![(
                "ChronoCraft".into(),
                "no model selected -- configure a local Ollama or MCP model to co-design system-dynamics models.".into(),
            )],
            help_tab: HelpTab::GetStarted,
            show_palette: false,
            palette_query: String::new(),
            show_shortcuts: false,
            show_licenses: false,
            show_about: false,
        }
    }

    pub fn goto(&mut self, page: Page, sub: usize) {
        self.page = page;
        self.subpage = sub;
    }

    fn persist(&mut self) {
        let _ = self.engine.save_db(&self.db_path);
    }

    fn sync_pending(&mut self) {
        for p in &self.engine.params {
            self.pending_params.entry(p.name.clone()).or_insert(p.value);
        }
        let known: Vec<String> = self.engine.params.iter().map(|p| p.name.clone()).collect();
        self.pending_params.retain(|k, _| known.contains(k));
    }

    fn run_simulation_cmd(&mut self) {
        let res = Command::RunSimulation {
            steps: Some(self.sim_steps),
            dt: Some(self.sim_dt),
        }
        .dispatch(&mut self.engine, &self.db_path);
        match res {
            Ok(val) => match serde_json::from_value::<TimeSeries>(val) {
                Ok(trace) => {
                    self.status_line = format!(
                        "Trace ready: {} steps, {} stocks",
                        trace.steps,
                        trace.series.len()
                    );
                    self.last_trace = Some(trace);
                }
                Err(e) => {
                    self.status_line = format!("Trace decode failed: {e}");
                }
            },
            Err(e) => {
                self.status_line = format!("Simulation failed: {e}");
            }
        }
    }

    fn run_mc_cmd(&mut self) {
        let res = Command::RunMonteCarlo {
            runs: self.mc_runs,
            seed: Some(self.mc_seed),
            shock_std: Some(self.mc_std),
        }
        .dispatch(&mut self.engine, &self.db_path);
        match res {
            Ok(val) => match serde_json::from_value::<MonteCarloResult>(val) {
                Ok(mc) => {
                    self.status_line = format!("Bands ready: {} runs, seed {}", mc.runs, mc.seed);
                    self.last_mc = Some(mc);
                }
                Err(e) => {
                    self.status_line = format!("Band decode failed: {e}");
                }
            },
            Err(e) => {
                self.status_line = format!("Monte Carlo failed: {e}");
            }
        }
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let theme = if self.dark_mode {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        };
        ctx.set_theme(theme);
        let mut style = (*ctx.style_of(theme)).clone();
        if self.dark_mode {
            style.visuals.override_text_color = Some(Color32::from_rgb(241, 245, 249));
            style.visuals.panel_fill = Color32::from_rgb(15, 23, 42);
        } else {
            style.visuals.override_text_color = Some(Color32::from_rgb(15, 23, 42));
            style.visuals.panel_fill = Color32::from_rgb(248, 250, 252);
        }
        ctx.set_style_of(theme, style);
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let zoom_delta = ctx.input(|i| {
            if i.modifiers.ctrl {
                i.smooth_scroll_delta.y
            } else {
                0.0
            }
        });
        if zoom_delta.abs() > 0.1 {
            let step = if zoom_delta > 0.0 { 0.1 } else { -0.1 };
            self.zoom_factor = (self.zoom_factor + step).clamp(0.75, 1.75);
            ctx.set_zoom_factor(self.zoom_factor);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.goto(Page::Help, 0);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::K)) {
            self.show_palette = !self.show_palette;
        }
    }

    pub fn render_menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save Model to SQLite").clicked() {
                    self.persist();
                    self.status_line = format!("Saved to {}", self.db_path.display());
                    ui.close();
                }
                if ui.button("Reset Population Seed").clicked() {
                    self.engine.load_preset(ModelPreset::Population);
                    self.persist();
                    self.sync_pending();
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Apply Pending Parameters").clicked() {
                    self.apply_pending_params();
                    ui.close();
                }
                if ui.button("Clear Cached Results").clicked() {
                    self.last_trace = None;
                    self.last_mc = None;
                    self.last_diff = None;
                    self.last_sweep = None;
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                if ui.button("Dark").clicked() {
                    self.dark_mode = true;
                    ui.close();
                }
                if ui.button("Light").clicked() {
                    self.dark_mode = false;
                    ui.close();
                }
                ui.separator();
                if ui.button("English").clicked() {
                    self.locale = AppLocale::En;
                    ui.close();
                }
                if ui.button("Deutsch").clicked() {
                    self.locale = AppLocale::De;
                    ui.close();
                }
            });
            ui.menu_button("Insert", |ui| {
                for (label, preset) in [
                    ("Preset: Population Growth", ModelPreset::Population),
                    ("Preset: Inventory Stock", ModelPreset::Inventory),
                    ("Preset: Epidemic SIR Loop", ModelPreset::EpidemicSir),
                ] {
                    if ui.button(label).clicked() {
                        self.engine.load_preset(preset);
                        self.persist();
                        self.sync_pending();
                        ui.close();
                    }
                }
            });
            ui.menu_button("Modify", |ui| {
                if ui.button("Double Shock Width").clicked() {
                    self.mc_std = (self.mc_std * 2.0).clamp(0.0, 1.0);
                    ui.close();
                }
            });
            ui.menu_button("Commands", |ui| {
                if ui.button("Run Deterministic Trace").clicked() {
                    self.run_simulation_cmd();
                    self.goto(Page::FanChart, 1);
                    ui.close();
                }
                if ui.button("Run Monte Carlo Bands").clicked() {
                    self.run_mc_cmd();
                    self.goto(Page::FanChart, 0);
                    ui.close();
                }
                if ui.button("Compare Saved Scenarios").clicked() {
                    self.goto(Page::Runs, 0);
                    ui.close();
                }
            });
            ui.menu_button("Window", |ui| {
                for (label, p, s) in [
                    ("Canvas Studio", Page::Canvas, 0),
                    ("Sliders & Parameters", Page::Sliders, 0),
                    ("Fan-Chart Viewer", Page::FanChart, 0),
                    ("Runs & Scenarios", Page::Runs, 0),
                    ("Chat", Page::Chat, 0),
                    ("Help (F1)", Page::Help, 0),
                ] {
                    if ui.button(label).clicked() {
                        self.goto(p, s);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Command Palette (Ctrl+K)").clicked() {
                    self.show_palette = true;
                    ui.close();
                }
                if ui.button("Keyboard Shortcuts").clicked() {
                    self.show_shortcuts = true;
                    ui.close();
                }
                if ui.button("Documentation (F1)").clicked() {
                    self.goto(Page::Help, 0);
                    ui.close();
                }
                if ui.button("Licenses").clicked() {
                    self.show_licenses = true;
                    ui.close();
                }
                if ui.button("About ChronoCraft").clicked() {
                    self.show_about = true;
                    ui.close();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_lbl = if self.dark_mode { "Light" } else { "Dark" };
                if ui.button(theme_lbl).clicked() {
                    self.dark_mode = !self.dark_mode;
                }
                let loc_lbl = self.locale.label();
                if ui.button(loc_lbl).clicked() {
                    self.locale = match self.locale {
                        AppLocale::En => AppLocale::De,
                        AppLocale::De => AppLocale::En,
                    };
                }
                ui.label(format!(
                    "Scenario: {} | Stocks: {} | Flows: {}",
                    self.engine.active_scenario,
                    self.engine.stocks.len(),
                    self.engine.flows.len()
                ));
            });
        });
        let _ = ctx;
    }

    fn apply_pending_params(&mut self) {
        self.sync_pending();
        let pending = self.pending_params.clone();
        let mut applied = 0;
        for (name, value) in &pending {
            let current = self
                .engine
                .params
                .iter()
                .find(|p| &p.name == name)
                .map(|p| p.value)
                .unwrap_or(f64::NAN);
            if (*value - current).abs() > f64::EPSILON {
                let res = Command::SetParam {
                    name: name.clone(),
                    value: *value,
                }
                .dispatch(&mut self.engine, &self.db_path);
                if res.is_ok() {
                    applied += 1;
                }
            }
        }
        self.persist();
        self.status_line = format!("Applied {applied} parameter changes");
    }

    pub fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.heading("ChronoCraft");
        ui.small("Dynamics & Scenario Lab");
        ui.separator();

        for wb in subapps::registry() {
            let subs = wb.subpages();
            let has_active = subs
                .iter()
                .any(|sp| sp.page == self.page && sp.sub == self.subpage);
            egui::CollapsingHeader::new(RichText::new(wb.title(self.locale)).strong())
                .id_salt(wb.id())
                .default_open(has_active || wb.id() == "canvas")
                .show(ui, |ui| {
                    for sp in subs {
                        let active = self.page == sp.page && self.subpage == sp.sub;
                        if ui.selectable_label(active, sp.label(self.locale)).clicked() {
                            self.goto(sp.page, sp.sub);
                        }
                    }
                });
        }
    }

    pub fn render_page(&mut self, ui: &mut egui::Ui) {
        match self.page {
            Page::Canvas => self.render_canvas(ui),
            Page::Sliders => self.render_sliders(ui),
            Page::FanChart => self.render_fanchart(ui),
            Page::Runs => self.render_runs(ui),
            Page::Chat => self.render_chat(ui),
            Page::Help => self.render_help_view(ui),
        }
    }

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let green = Color32::from_rgb(34, 197, 94);
        let slate = Color32::from_rgb(100, 116, 139);
        let cyan = Color32::from_rgb(56, 189, 248);
        let amber = Color32::from_rgb(245, 158, 11);

        ui.horizontal_wrapped(|ui| {
            ui.heading("1. Canvas: Stocks, Flows & Feedback");
            for (lbl, p) in [
                ("Population", ModelPreset::Population),
                ("Inventory", ModelPreset::Inventory),
                ("Epidemic SIR", ModelPreset::EpidemicSir),
            ] {
                if ui
                    .selectable_label(self.engine.active_scenario == p.as_str(), lbl)
                    .clicked()
                {
                    self.engine.load_preset(p);
                    self.persist();
                    self.sync_pending();
                }
            }
            if ui.button("Run Trace").clicked() {
                self.run_simulation_cmd();
            }
        });
        ui.separator();

        if self.subpage == 1 {
            ui.group(|ui| {
                ui.strong("Add Stock");
                ui.horizontal_wrapped(|ui| {
                    ui.label("ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_stock_id)
                            .hint_text("Stock ID")
                            .desired_width(90.0),
                    );
                    ui.label("Label:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_stock_label)
                            .hint_text("Stock Label")
                            .desired_width(110.0),
                    );
                    ui.label("Initial:");
                    ui.add(
                        egui::Slider::new(&mut self.new_stock_initial, 0.0..=100_000.0)
                            .text("Initial stock value"),
                    );
                    if ui.button("Add Stock").clicked() {
                        let res = Command::AddStock {
                            id: self.new_stock_id.clone(),
                            label: self.new_stock_label.clone(),
                            initial: self.new_stock_initial,
                            x: 360.0,
                            y: 200.0,
                        }
                        .dispatch(&mut self.engine, &self.db_path);
                        match res {
                            Ok(_) => {
                                self.status_line = format!("Added stock {}", self.new_stock_id);
                            }
                            Err(e) => {
                                self.status_line = format!("Add stock failed: {e}");
                            }
                        }
                    }
                });
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("Add Flow (empty side = model boundary)");
                ui.horizontal_wrapped(|ui| {
                    ui.label("ID:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_flow_id)
                            .hint_text("Flow ID")
                            .desired_width(90.0),
                    );
                    ui.label("From:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_flow_from)
                            .hint_text("Source stock or empty")
                            .desired_width(110.0),
                    );
                    ui.label("To:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_flow_to)
                            .hint_text("Target stock or empty")
                            .desired_width(110.0),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Rate:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_flow_expr)
                            .hint_text("Rate expression e.g. growth * pop")
                            .desired_width(260.0),
                    );
                    if ui.button("Add Flow").clicked() {
                        let from = clean_opt(&self.new_flow_from);
                        let to = clean_opt(&self.new_flow_to);
                        let res = Command::AddFlow {
                            id: self.new_flow_id.clone(),
                            label: self.new_flow_label.clone(),
                            from_stock: from,
                            to_stock: to,
                            rate_expr: self.new_flow_expr.clone(),
                            x: 200.0,
                            y: 260.0,
                        }
                        .dispatch(&mut self.engine, &self.db_path);
                        match res {
                            Ok(_) => {
                                self.status_line = format!("Added flow {}", self.new_flow_id);
                            }
                            Err(e) => {
                                self.status_line = format!("Add flow failed: {e}");
                            }
                        }
                    }
                });
            });
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.strong("Add Feedback Link");
                ui.horizontal_wrapped(|ui| {
                    ui.label("From Stock:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.link_from)
                            .hint_text("Stock ID")
                            .desired_width(110.0),
                    );
                    ui.label("To Flow:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.link_to)
                            .hint_text("Flow ID")
                            .desired_width(110.0),
                    );
                    if ui.button("Add Link").clicked() {
                        let res = Command::AddLink {
                            from_node: self.link_from.clone(),
                            to_flow: self.link_to.clone(),
                        }
                        .dispatch(&mut self.engine, &self.db_path);
                        match res {
                            Ok(_) => {
                                self.status_line = "Feedback link recorded".to_string();
                            }
                            Err(e) => {
                                self.status_line = format!("Add link failed: {e}");
                            }
                        }
                    }
                });
            });
            ui.add_space(6.0);
        }

        ui.horizontal_wrapped(|ui| {
            ui.strong("Stocks:");
            for s in &self.engine.stocks {
                ui.colored_label(green, format!("[{}] {} = {:.2}", s.label, s.id, s.initial));
            }
            ui.separator();
            ui.strong("Flows:");
            for f in &self.engine.flows {
                ui.monospace(format!("{}: {}", f.id, f.rate_expr));
            }
        });
        ui.add_space(6.0);

        let canvas_size = Vec2::new(ui.available_width().max(640.0), 380.0);
        let (rect, _) = ui.allocate_exact_size(canvas_size, Sense::hover());

        // Interaction pass: draggable nodes (view layout on the canvas).
        let mut drags: Vec<(bool, Vec2, bool)> = Vec::new();
        for s in &self.engine.stocks {
            let r = Rect::from_center_size(
                Pos2::new(rect.left() + s.x, rect.top() + s.y),
                Vec2::new(128.0, 56.0),
            );
            let resp = ui.allocate_rect(r, Sense::drag());
            drags.push((true, resp.drag_delta(), resp.dragged()));
        }
        for f in &self.engine.flows {
            let r = Rect::from_center_size(
                Pos2::new(rect.left() + f.x, rect.top() + f.y),
                Vec2::new(120.0, 48.0),
            );
            let resp = ui.allocate_rect(r, Sense::drag());
            drags.push((false, resp.drag_delta(), resp.dragged()));
        }
        for (idx, (is_stock, delta, dragged)) in drags.into_iter().enumerate() {
            if !dragged {
                continue;
            }
            if is_stock {
                if let Some(s) = self.engine.stocks.get_mut(idx) {
                    s.x = (s.x + delta.x).clamp(60.0, 1200.0);
                    s.y = (s.y + delta.y).clamp(30.0, 800.0);
                }
            } else if let Some(f) = self.engine.flows.get_mut(idx - self.engine.stocks.len()) {
                f.x = (f.x + delta.x).clamp(60.0, 1200.0);
                f.y = (f.y + delta.y).clamp(30.0, 800.0);
            }
        }

        let painter = ui.painter_at(rect);
        let bg = if self.dark_mode {
            Color32::from_rgb(9, 14, 28)
        } else {
            Color32::from_rgb(241, 245, 249)
        };
        painter.rect_filled(rect, 8.0, bg);

        let pos_of = |x: f32, y: f32| -> Pos2 { Pos2::new(rect.left() + x, rect.top() + y) };
        let stock_center = |id: &str, eng: &ChronoEngine| -> Option<Pos2> {
            eng.stocks
                .iter()
                .find(|s| s.id == id)
                .map(|s| pos_of(s.x, s.y))
        };
        let flow_center = |id: &str, eng: &ChronoEngine| -> Option<Pos2> {
            eng.flows
                .iter()
                .find(|f| f.id == id)
                .map(|f| pos_of(f.x, f.y))
        };

        for f in &self.engine.flows {
            let fc = pos_of(f.x, f.y);
            if let Some(from) = &f.from_stock {
                if let Some(sc) = stock_center(from, &self.engine) {
                    painter.line_segment([sc, fc], Stroke::new(2.5, green));
                }
            } else {
                painter.line_segment([Pos2::new(fc.x - 70.0, fc.y), fc], Stroke::new(2.0, slate));
            }
            if let Some(to) = &f.to_stock {
                if let Some(sc) = stock_center(to, &self.engine) {
                    painter.line_segment([fc, sc], Stroke::new(2.5, green));
                }
            } else {
                painter.line_segment([fc, Pos2::new(fc.x + 70.0, fc.y)], Stroke::new(2.0, slate));
            }
        }
        for l in &self.engine.links {
            if let (Some(a), Some(b)) = (
                stock_center(&l.from_node, &self.engine),
                flow_center(&l.to_flow, &self.engine),
            ) {
                painter.line_segment([a, b], Stroke::new(1.2, amber));
            }
        }

        let txt_col = if self.dark_mode {
            Color32::WHITE
        } else {
            Color32::from_rgb(15, 23, 42)
        };
        for s in &self.engine.stocks {
            let center = pos_of(s.x, s.y);
            let node_rect = Rect::from_center_size(center, Vec2::new(128.0, 56.0));
            painter.rect_filled(node_rect, 6.0, Color32::from_rgb(20, 83, 45));
            painter.rect_stroke(
                node_rect,
                6.0,
                Stroke::new(2.0, green),
                egui::StrokeKind::Outside,
            );
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                format!("{}\n{} = {:.1}", s.label, s.id, s.initial),
                egui::FontId::monospace(11.0),
                txt_col,
            );
        }
        for f in &self.engine.flows {
            let center = pos_of(f.x, f.y);
            let node_rect = Rect::from_center_size(center, Vec2::new(120.0, 48.0));
            let fill = if self.dark_mode {
                Color32::from_rgb(30, 41, 59)
            } else {
                Color32::from_rgb(226, 232, 240)
            };
            painter.rect_filled(node_rect, 6.0, fill);
            painter.rect_stroke(
                node_rect,
                6.0,
                Stroke::new(2.0, cyan),
                egui::StrokeKind::Outside,
            );
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                format!("{}\n{}", f.label, f.rate_expr),
                egui::FontId::monospace(10.0),
                txt_col,
            );
        }
    }

    fn render_sliders(&mut self, ui: &mut egui::Ui) {
        ui.heading("2. Lumped Parameters & Solver Settings");
        ui.separator();
        self.sync_pending();

        if self.subpage == 1 {
            ui.group(|ui| {
                ui.strong("Solver & Monte Carlo run settings");
                ui.add(egui::Slider::new(&mut self.sim_steps, 10..=5000).text("Trace steps"));
                ui.add(egui::Slider::new(&mut self.sim_dt, 0.005..=0.2).text("Step size dt"));
                ui.add(egui::Slider::new(&mut self.mc_runs, 10..=10000).text("Monte Carlo runs"));
                ui.add(egui::Slider::new(&mut self.mc_std, 0.0..=1.0).text("Shock width"));
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut self.mc_seed, 0..=1_000_000).text("RNG seed"));
                });
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Run Deterministic Trace").clicked() {
                        self.run_simulation_cmd();
                    }
                    if ui.button("Run Monte Carlo Bands").clicked() {
                        self.run_mc_cmd();
                    }
                });
                ui.small(&self.status_line);
            });
            return;
        }

        ui.group(|ui| {
            ui.strong("Lumped parameters (move sliders, then Apply)");
            let names: Vec<String> = self.engine.params.iter().map(|p| p.name.clone()).collect();
            for name in names {
                let (min, max, current) = self
                    .engine
                    .params
                    .iter()
                    .find(|p| p.name == name)
                    .map(|p| (p.min, p.max, p.value))
                    .unwrap_or((0.0, 1.0, 0.0));
                let pending = self.pending_params.entry(name.clone()).or_insert(current);
                ui.horizontal(|ui| {
                    ui.monospace(format!("{name} = {current:.4}"));
                    let lo = min.min(*pending).min(current);
                    let hi = max.max(*pending).max(current);
                    let (lo, hi) = if lo < hi {
                        (lo, hi)
                    } else {
                        (lo - 1.0, hi + 1.0)
                    };
                    ui.add(egui::Slider::new(pending, lo..=hi).text(&name));
                });
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("Apply Parameters").clicked() {
                    self.apply_pending_params();
                }
                if ui.button("Reset Pending").clicked() {
                    self.pending_params.clear();
                    self.sync_pending();
                }
            });
            ui.small(&self.status_line);
        });
    }

    fn render_fanchart(&mut self, ui: &mut egui::Ui) {
        ui.heading("3. Fan-Chart Viewer: p10/p50/p90 Bands");
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_label("Stock")
                .selected_text(&self.fan_stock)
                .show_ui(ui, |ui| {
                    for s in &self.engine.stocks {
                        ui.selectable_value(&mut self.fan_stock, s.id.clone(), &s.label);
                    }
                });
            if ui.button("Run Trace").clicked() {
                self.run_simulation_cmd();
            }
            if ui.button("Run Monte Carlo").clicked() {
                self.run_mc_cmd();
            }
        });
        ui.separator();

        if self.subpage == 1 {
            ui.group(|ui| {
                ui.strong(format!("Deterministic RK4 trace: {}", self.fan_stock));
                match &self.last_trace {
                    Some(trace) => {
                        paint_series(
                            ui,
                            self.dark_mode,
                            trace.times.clone(),
                            series_of(trace, &self.fan_stock),
                            false,
                            None,
                        );
                    }
                    None => {
                        ui.label("No trace yet -- press Run Trace.");
                    }
                }
            });
            return;
        }

        ui.group(|ui| {
            ui.strong(format!("Monte Carlo percentile bands: {}", self.fan_stock));
            match &self.last_mc {
                Some(mc) => {
                    let band = mc.bands.iter().find(|b| b.stock == self.fan_stock);
                    match band {
                        Some(b) => {
                            ui.label(format!(
                                "runs: {} | seed: {} | shock width: {:.3} | end p50: {:.2}",
                                mc.runs,
                                mc.seed,
                                mc.shock_std,
                                b.p50.last().copied().unwrap_or(0.0)
                            ));
                            paint_series(
                                ui,
                                self.dark_mode,
                                mc.times.clone(),
                                b.p50.clone(),
                                true,
                                Some((b.p10.clone(), b.p90.clone())),
                            );
                        }
                        None => {
                            ui.label(format!(
                                "Stock '{}' is not in the cached bands -- run again after model edits.",
                                self.fan_stock
                            ));
                        }
                    }
                }
                None => {
                    ui.label("No bands yet -- press Run Monte Carlo.");
                }
            }
        });
    }

    fn render_runs(&mut self, ui: &mut egui::Ui) {
        ui.heading("4. Runs: Scenarios, Compare & Sensitivity");
        ui.separator();

        if self.subpage == 1 {
            ui.group(|ui| {
                ui.strong("One-parameter sensitivity sweep");
                ui.horizontal_wrapped(|ui| {
                    ui.label("Param:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.sweep_param)
                            .hint_text("Parameter name")
                            .desired_width(110.0),
                    );
                    ui.label("Target:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.sweep_target)
                            .hint_text("Stock id")
                            .desired_width(110.0),
                    );
                });
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.sweep_from, -1_000_000.0..=1_000_000.0)
                            .text("Sweep from"),
                    );
                    ui.add(
                        egui::Slider::new(&mut self.sweep_to, -1_000_000.0..=1_000_000.0)
                            .text("Sweep to"),
                    );
                    ui.add(egui::Slider::new(&mut self.sweep_steps, 2..=21).text("grid"));
                    if ui.button("Run Sweep").clicked() {
                        let res = Command::SensitivitySweep {
                            param: self.sweep_param.clone(),
                            from: self.sweep_from,
                            to: self.sweep_to,
                            steps: self.sweep_steps,
                            target: self.sweep_target.clone(),
                        }
                        .dispatch(&mut self.engine, &self.db_path);
                        match res {
                            Ok(val) => match serde_json::from_value::<SensitivityResult>(val) {
                                Ok(s) => {
                                    self.status_line = format!(
                                        "Sweep ready: {} over {} points",
                                        s.param,
                                        s.values.len()
                                    );
                                    self.last_sweep = Some(s);
                                }
                                Err(e) => {
                                    self.status_line = format!("Sweep decode failed: {e}");
                                }
                            },
                            Err(e) => {
                                self.status_line = format!("Sweep failed: {e}");
                            }
                        }
                    }
                });
            });
            ui.add_space(6.0);
            match &self.last_sweep {
                Some(s) => {
                    paint_series(
                        ui,
                        self.dark_mode,
                        s.values.clone(),
                        s.end_states.clone(),
                        false,
                        None,
                    );
                    egui::Grid::new("sweep_grid").striped(true).show(ui, |ui| {
                        ui.strong(&s.param);
                        ui.strong(format!("end({})", s.target));
                        ui.end_row();
                        for (v, e) in s.values.iter().zip(&s.end_states) {
                            ui.monospace(format!("{v:.4}"));
                            ui.monospace(format!("{e:.4}"));
                            ui.end_row();
                        }
                    });
                }
                None => {
                    ui.label("No sweep yet -- configure the grid and press Run Sweep.");
                }
            }
            return;
        }

        ui.group(|ui| {
            ui.strong("Save & list scenarios");
            ui.horizontal_wrapped(|ui| {
                ui.label("Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.scenario_name)
                        .hint_text("Scenario name")
                        .desired_width(140.0),
                );
                if ui.button("Save Scenario").clicked() {
                    let res = Command::SaveScenario {
                        name: self.scenario_name.clone(),
                    }
                    .dispatch(&mut self.engine, &self.db_path);
                    match res {
                        Ok(_) => {
                            self.status_line = format!("Saved scenario {}", self.scenario_name);
                        }
                        Err(e) => {
                            self.status_line = format!("Save failed: {e}");
                        }
                    }
                }
                if ui.button("Refresh List").clicked() {
                    let res = Command::ListScenarios.dispatch(&mut self.engine, &self.db_path);
                    match res {
                        Ok(val) => {
                            let names: Vec<String> = val
                                .get("scenarios")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(str::to_string))
                                        .collect()
                                })
                                .unwrap_or_default();
                            self.last_list = names;
                        }
                        Err(e) => {
                            self.status_line = format!("List failed: {e}");
                        }
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.strong("Persisted:");
                for n in &self.last_list {
                    ui.monospace(n);
                }
                if self.last_list.is_empty() {
                    ui.small("press Refresh List");
                }
            });
        });
        ui.add_space(6.0);
        ui.group(|ui| {
            ui.strong("Baseline vs variant end-state diff");
            ui.horizontal_wrapped(|ui| {
                ui.label("Baseline:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.cmp_base)
                        .hint_text("Baseline name")
                        .desired_width(120.0),
                );
                ui.label("Variant:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.cmp_var)
                        .hint_text("Variant name")
                        .desired_width(120.0),
                );
                if ui.button("Compare").clicked() {
                    let res = Command::CompareScenarios {
                        baseline: self.cmp_base.clone(),
                        variant: self.cmp_var.clone(),
                    }
                    .dispatch(&mut self.engine, &self.db_path);
                    match res {
                        Ok(val) => match serde_json::from_value::<ScenarioDiff>(val) {
                            Ok(d) => {
                                self.last_diff = Some(d);
                            }
                            Err(e) => {
                                self.status_line = format!("Diff decode failed: {e}");
                            }
                        },
                        Err(e) => {
                            self.status_line = format!("Compare failed: {e}");
                        }
                    }
                }
            });
            match &self.last_diff {
                Some(d) => {
                    egui::Grid::new("diff_grid").striped(true).show(ui, |ui| {
                        ui.strong("Stock");
                        ui.strong("Baseline");
                        ui.strong("Variant");
                        ui.strong("Delta");
                        ui.end_row();
                        for row in &d.end {
                            ui.monospace(&row.stock);
                            ui.monospace(format!("{:.4}", row.baseline));
                            ui.monospace(format!("{:.4}", row.variant));
                            ui.monospace(format!("{:+.4}", row.delta));
                            ui.end_row();
                        }
                    });
                }
                None => {
                    ui.label("No diff yet -- save two scenarios and press Compare.");
                }
            }
        });
    }

    fn render_chat(&mut self, ui: &mut egui::Ui) {
        ui.heading("ChronoCraft Co-Designer Chat");
        ui.label("no model selected -- connect an MCP client or local Ollama endpoint to draft system-dynamics models from natural language.");
        ui.separator();
        for (who, msg) in &self.chat_history {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("{who}:"));
                ui.label(msg);
            });
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.chat_input)
                    .hint_text("Ask about stocks, flows, bands, or scenarios...")
                    .desired_width(420.0),
            );
            if ui.button("Send").clicked() && !self.chat_input.trim().is_empty() {
                let q = self.chat_input.trim().to_string();
                self.chat_input.clear();
                self.chat_history.push(("You".into(), q));
                self.chat_history.push((
                    "ChronoCraft".into(),
                    "no model selected -- use the 13 MCP tools (`run_simulation`, `run_monte_carlo`, `compare_scenarios`, `sensitivity_sweep`) from your agent IDE.".into(),
                ));
            }
        });
    }

    fn render_help_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("ChronoCraft Documentation & Reference (F1)");
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (HelpTab::GetStarted, "Get Started"),
                (HelpTab::CanvasStocks, "Canvas & Stocks"),
                (HelpTab::SlidersParams, "Sliders & Solver"),
                (HelpTab::BandsCompare, "Bands & Compare"),
                (HelpTab::CliMcp, "CLI & MCP"),
                (HelpTab::TechnicalInfo, "Technical Info"),
            ] {
                if ui.selectable_label(self.help_tab == tab, label).clicked() {
                    self.help_tab = tab;
                }
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let _ = HELP_MD;
            crate::help_docs::render_help_tab_content(self, ui);
        });
    }

    fn render_modals(&mut self, ctx: &egui::Context) {
        if self.show_palette {
            let mut open = self.show_palette;
            let mut nav_target: Option<(Page, usize)> = None;
            egui::Window::new("Command Palette")
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.palette_query)
                            .hint_text("Filter commands..."),
                    );
                    for (label, page, sub) in [
                        ("Canvas: Model Graph", Page::Canvas, 0),
                        ("Canvas: Presets & Builder", Page::Canvas, 1),
                        ("Parameter Sliders", Page::Sliders, 0),
                        ("Solver Settings", Page::Sliders, 1),
                        ("Fan-Chart Bands", Page::FanChart, 0),
                        ("Deterministic Trace", Page::FanChart, 1),
                        ("Scenarios & Compare", Page::Runs, 0),
                        ("Sensitivity Sweep", Page::Runs, 1),
                        ("Help Documentation (F1)", Page::Help, 0),
                    ] {
                        if (self.palette_query.is_empty()
                            || label
                                .to_ascii_lowercase()
                                .contains(&self.palette_query.to_ascii_lowercase()))
                            && ui.button(label).clicked()
                        {
                            nav_target = Some((page, sub));
                        }
                    }
                });
            self.show_palette = open;
            if let Some((page, sub)) = nav_target {
                self.goto(page, sub);
                self.show_palette = false;
            }
        }
        if self.show_shortcuts {
            egui::Window::new("Keyboard Shortcuts")
                .open(&mut self.show_shortcuts)
                .show(ctx, |ui| {
                    ui.label("F1: Open 6-Tab Help");
                    ui.label("Ctrl+K: Toggle Command Palette");
                    ui.label("Ctrl+MouseWheel: Zoom UI (0.75x .. 1.75x)");
                });
        }
        if self.show_licenses {
            egui::Window::new("Licenses & Attribution")
                .open(&mut self.show_licenses)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(ATTRIBUTION_MD);
                    });
                });
        }
        if self.show_about {
            egui::Window::new("About ChronoCraft")
                .open(&mut self.show_about)
                .show(ctx, |ui| {
                    ui.heading(format!("ChronoCraft v{APP_VERSION}"));
                    ui.label("Pure-Rust System-Dynamics and Monte Carlo Scenario Simulator: stocks-and-flows models, RK4 solver, percentile fan charts, scenario compare, and sensitivity sweeps.");
                });
        }
    }
}

impl eframe::App for ChronoCraftApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_theme(&ctx);
        self.handle_shortcuts(&ctx);

        egui::Panel::top("chronocraft_top_menu").show(ui, |ui| {
            self.render_menu_bar(&ctx, ui);
        });
        egui::Panel::bottom("chronocraft_status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_line);
            });
        });
        egui::Panel::left("chronocraft_sidebar")
            .default_size(225.0)
            .show(ui, |ui| {
                self.render_sidebar(ui);
            });
        egui::CentralPanel::default().show(ui, |ui| {
            self.render_page(ui);
        });

        self.render_modals(&ctx);
    }
}

fn clean_opt(raw: &str) -> Option<String> {
    let t = raw.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn series_of(trace: &TimeSeries, stock: &str) -> Vec<f64> {
    trace
        .series
        .iter()
        .find(|s| s.stock == stock)
        .map(|s| s.values.clone())
        .unwrap_or_default()
}

/// Paint a line series, optionally with a p10/p90 shaded band behind it.
fn paint_series(
    ui: &mut egui::Ui,
    dark_mode: bool,
    xs: Vec<f64>,
    ys: Vec<f64>,
    is_band_median: bool,
    band: Option<(Vec<f64>, Vec<f64>)>,
) {
    if xs.len() < 2 || ys.len() != xs.len() {
        ui.label("Not enough points to paint.");
        return;
    }
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in &ys {
        lo = lo.min(*v);
        hi = hi.max(*v);
    }
    if let Some((ref p10, ref p90)) = band {
        for v in p10.iter().chain(p90.iter()) {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
    }
    if !lo.is_finite() || !hi.is_finite() || (hi - lo).abs() < 1e-12 {
        ui.label("Series is flat -- nothing to paint.");
        return;
    }
    let x0 = xs.first().copied().unwrap_or(0.0);
    let x1 = xs.last().copied().unwrap_or(1.0);
    let span_x = (x1 - x0).abs().max(1e-12);

    let height: f32 = 260.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().max(600.0), height),
        Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let bg = if dark_mode {
        Color32::from_rgb(9, 14, 28)
    } else {
        Color32::from_rgb(241, 245, 249)
    };
    painter.rect_filled(rect, 6.0, bg);

    let map = |x: f64, y: f64| -> Pos2 {
        let fx = ((x - x0) / span_x) as f32;
        let fy = ((y - lo) / (hi - lo)) as f32;
        Pos2::new(
            rect.left() + 12.0 + fx * (rect.width() - 24.0),
            rect.bottom() - 12.0 - fy * (height - 24.0),
        )
    };

    if let Some((p10, p90)) = band {
        let fill = Color32::from_rgba_premultiplied(56, 130, 200, 70);
        for (i, x) in xs.iter().enumerate() {
            let a = p10.get(i).copied().unwrap_or(lo);
            let b = p90.get(i).copied().unwrap_or(hi);
            painter.line_segment([map(*x, a), map(*x, b)], Stroke::new(3.0, fill));
        }
    }
    let line_col = if is_band_median {
        Color32::from_rgb(34, 197, 94)
    } else {
        Color32::from_rgb(56, 189, 248)
    };
    let mut prev: Option<Pos2> = None;
    for (x, y) in xs.iter().zip(&ys) {
        let pt = map(*x, *y);
        if let Some(p) = prev {
            painter.line_segment([p, pt], Stroke::new(2.0, line_col));
        }
        prev = Some(pt);
    }
    painter.text(
        Pos2::new(rect.left() + 16.0, rect.top() + 14.0),
        egui::Align2::LEFT_TOP,
        format!("max {hi:.2} / min {lo:.2}"),
        egui::FontId::monospace(11.0),
        if dark_mode {
            Color32::from_rgb(203, 213, 225)
        } else {
            Color32::from_rgb(51, 65, 85)
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;

    fn temp_app() -> ChronoCraftApp {
        let mut p = std::env::temp_dir();
        p.push(format!("chronocraft_app_test_{}.db", std::process::id()));
        ChronoCraftApp::with_db_path(p)
    }

    #[test]
    fn subapp_registry_exposes_all_six_workbenches() {
        let reg = subapps::registry();
        assert_eq!(reg.len(), 6);
        let total_subpages: usize = reg.iter().map(|w| w.subpages().len()).sum();
        assert_eq!(total_subpages, 10);
    }

    #[test]
    fn headless_egui_kittest_renders_all_workbench_subpages() {
        let mut app = temp_app();
        for wb in subapps::registry() {
            for sp in wb.subpages() {
                app.goto(sp.page, sp.sub);
                let mut harness = Harness::new_ui(|ui| {
                    let ctx = ui.ctx().clone();
                    app.render_menu_bar(&ctx, ui);
                    app.render_sidebar(ui);
                    app.render_page(ui);
                });
                harness.run();
            }
        }
        let _ = std::fs::remove_file(&app.db_path);
    }

    #[test]
    fn headless_egui_kittest_renders_all_six_help_tabs_and_both_themes() {
        let mut app = temp_app();
        app.goto(Page::Help, 0);
        for tab in [
            HelpTab::GetStarted,
            HelpTab::CanvasStocks,
            HelpTab::SlidersParams,
            HelpTab::BandsCompare,
            HelpTab::CliMcp,
            HelpTab::TechnicalInfo,
        ] {
            app.help_tab = tab;
            app.dark_mode = !app.dark_mode;
            let mut harness = Harness::new_ui(|ui| {
                let ctx = ui.ctx().clone();
                app.apply_theme(&ctx);
                app.render_page(ui);
            });
            harness.run();
        }
        let _ = std::fs::remove_file(&app.db_path);
    }
}
