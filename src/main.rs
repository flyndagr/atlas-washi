#![forbid(unsafe_code)]
use atlas::{Vault, tags, wiki_links};
use eframe::egui::{self, Color32 as C, RichText as R, *};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

mod washi;
use washi::{ACCENT, GREEN, INK, LINE, MUTED, PANEL, PAPER};

#[derive(PartialEq)]
enum Mode {
    Notes,
    Canvas,
}
struct Atlas {
    vault: Vault,
    selected: Option<usize>,
    draft: String,
    dirty: bool,
    changed: Instant,
    query: String,
    mode: Mode,
    editing: bool,
    error: Option<String>,
    status: String,
    new_note: bool,
    new_title: String,
    new_note_error: Option<String>,
    focus_new: bool,
    search_focus: bool,
    board_dirty: bool,
    board_changed: Instant,
    screenshot: Option<PathBuf>,
    frames: u32,
    quit_after: bool,
    paper_texture: TextureHandle,
    panels: atlas::view::Panels,
    fountain: bool,
    appearance: washi::Appearance,
    rename_dialog: bool,
    rename_title: String,
    trash_dialog: bool,
    focus_editor: bool,
    pending_selection: Option<(usize, usize)>,
}

impl Atlas {
    fn new(
        cc: &eframe::CreationContext<'_>,
        root: PathBuf,
        canvas: bool,
        screenshot: Option<PathBuf>,
        quit_after: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        cc.egui_ctx.set_fonts(washi::fonts());
        let appearance_path = root.join(".atlas/appearance.json");
        let mut appearance: washi::Appearance = std::fs::read(&appearance_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        appearance.grain = if appearance.grain.is_finite() {
            appearance.grain.clamp(0., 1.)
        } else {
            0.7
        };
        let paper_texture = washi::texture(&cc.egui_ctx, appearance.grain);
        let mut style = (*cc.egui_ctx.style_of(Theme::Light)).clone();
        style.visuals = Visuals::light();
        style.visuals.override_text_color = Some(INK);
        style.visuals.panel_fill = PAPER;
        style.visuals.window_fill = PAPER;
        style.visuals.extreme_bg_color = PAPER;
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, LINE);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(3);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(3);
        style.visuals.text_cursor.stroke = Stroke::new(1.5, ACCENT);
        style.visuals.faint_bg_color = PANEL;
        style.visuals.selection.bg_fill = C::from_rgb(222, 222, 195);
        style.visuals.selection.stroke = Stroke::new(1., GREEN);
        style.visuals.widgets.inactive.bg_fill = PANEL;
        style.visuals.widgets.inactive.weak_bg_fill = PANEL;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1., LINE);
        style.visuals.widgets.hovered.bg_fill = C::from_rgb(229, 221, 200);
        style.visuals.widgets.hovered.weak_bg_fill = C::from_rgb(229, 221, 200);
        style.spacing.item_spacing = vec2(10., 9.);
        style.spacing.button_padding = vec2(12., 7.);
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(14.));
        style
            .text_styles
            .insert(TextStyle::Heading, washi::serif(34.));
        cc.egui_ctx.set_style_of(Theme::Light, style);
        cc.egui_ctx.set_theme(Theme::Light);
        let mut vault = Vault::open(root).map_err(std::io::Error::other)?;
        vault.seed().map_err(std::io::Error::other)?;
        let selected = vault.find("Start here").or(if vault.notes.is_empty() {
            None
        } else {
            Some(0)
        });
        let draft = selected
            .and_then(|i| vault.notes.get(i))
            .map(|n| n.body.clone())
            .unwrap_or_default();
        Ok(Self {
            vault,
            selected,
            draft,
            dirty: false,
            changed: Instant::now(),
            query: String::new(),
            mode: if canvas { Mode::Canvas } else { Mode::Notes },
            editing: false,
            error: None,
            status: "All changes saved".into(),
            new_note: false,
            new_title: String::new(),
            new_note_error: None,
            focus_new: false,
            search_focus: false,
            board_dirty: false,
            board_changed: Instant::now(),
            screenshot,
            frames: 0,
            quit_after,
            paper_texture,
            panels: {
                let mut panels = atlas::view::Panels::default();
                if std::env::args().any(|a| a == "--focus") {
                    panels.toggle_focus();
                }
                panels
            },
            fountain: appearance.fountain,
            appearance,
            rename_dialog: false,
            rename_title: String::new(),
            trash_dialog: false,
            focus_editor: false,
            pending_selection: None,
        })
    }
    fn select_fresh(&mut self, index: Option<usize>) {
        self.selected = index;
        self.editing = false;
        self.draft = index
            .map(|i| self.vault.notes[i].body.clone())
            .unwrap_or_default();
        self.dirty = false;
        self.query.clear();
    }
    fn import_folder(&mut self) {
        if !self.save() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Import folder — Markdown and local assets")
            .pick_folder()
        {
            match self.vault.import_folder(&path) {
                Ok((prefix, count)) => {
                    let i = self
                        .vault
                        .notes
                        .iter()
                        .position(|n| n.title.starts_with(&format!("{prefix}/")));
                    self.select_fresh(i);
                    self.mode = Mode::Notes;
                    self.status =
                        format!("Imported {count} notes with their folder structure and assets");
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
    fn move_to_trash(&mut self) {
        if !self.save() || !self.save_board() {
            return;
        }
        if let Some(i) = self.selected {
            match self.vault.trash_note(i) {
                Ok(()) => {
                    let next = if self.vault.notes.is_empty() {
                        None
                    } else {
                        Some(i.min(self.vault.notes.len() - 1))
                    };
                    self.select_fresh(next);
                    self.status = "Moved to Trash. Restore it from Files → Trash.".into();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
    fn editor_id(&self) -> Id {
        Id::new((
            "markdown-editor",
            self.selected.map(|i| self.vault.notes[i].path.clone()),
        ))
    }
    fn format_text(&mut self, ctx: &Context, format: atlas::editing::Format) {
        use egui::text::{CCursor, CCursorRange};
        let id = self.editor_id();
        let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
        let range = state
            .cursor
            .char_range()
            .unwrap_or(CCursorRange::one(CCursor::new(self.draft.chars().count())));
        let a = range.primary.index.min(range.secondary.index);
        let b = range.primary.index.max(range.secondary.index);
        let mut undo = state.undoer();
        undo.add_undo(&(range, self.draft.clone()));
        let (a, b) = atlas::editing::apply(&mut self.draft, a.into(), b.into(), format);
        state.set_undoer(undo);
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
        state.store(ctx, id);
        self.focus_editor = true;
        self.pending_selection = Some((a, b));
        self.touch();
    }
    fn editor(&mut self, ui: &mut Ui) {
        use atlas::editing::Format;
        let ctx = ui.ctx().clone();
        let id = self.editor_id();
        if ctx.memory(|m| m.has_focus(id)) {
            if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::B)) {
                self.format_text(&ctx, Format::Bold);
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::I)) {
                self.format_text(&ctx, Format::Italic);
            }
        }
        if ctx.memory(|m| m.has_focus(id))
            && ctx.input(|i| i.key_pressed(Key::Enter) && i.modifiers == Modifiers::NONE)
        {
            let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
            if let Some(range) = state.cursor.char_range()
                && range.primary.index == range.secondary.index
            {
                let mut next = self.draft.clone();
                if let Some(cursor) =
                    atlas::editing::list_enter(&mut next, range.primary.index.into())
                {
                    ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
                    let mut undo = state.undoer();
                    undo.add_undo(&(range, self.draft.clone()));
                    state.set_undoer(undo);
                    state.store(&ctx, id);
                    self.draft = next;
                    self.focus_editor = true;
                    self.pending_selection = Some((cursor, cursor));
                    self.touch();
                }
            }
        }
        let r = ui.add(
            TextEdit::multiline(&mut self.draft)
                .id(id)
                .text_color(self.appearance.kind.ink())
                .font(if self.fountain {
                    washi::script(32.)
                } else {
                    washi::serif(24.)
                })
                .desired_width(f32::INFINITY)
                .desired_rows(20)
                .frame(Frame::NONE)
                .hint_text("A thought, a question, a beginning…"),
        );
        if self.focus_editor {
            r.request_focus();
            if let Some((a, b)) = self.pending_selection.take() {
                let mut state = egui::text_edit::TextEditState::load(&ctx, id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(a),
                        egui::text::CCursor::new(b),
                    )));
                state.store(&ctx, id);
            }
            self.focus_editor = false;
        }
        if r.changed() {
            self.touch();
        }
    }
    fn import_files(&mut self) {
        if !self.save() {
            return;
        }
        if let Some(files) = rfd::FileDialog::new()
            .set_title("Import Markdown files")
            .add_filter("Markdown", &["md", "markdown", "MD"])
            .pick_files()
        {
            let old_title = self.selected.map(|i| self.vault.notes[i].title.clone());
            let mut last_title = None;
            let mut count = 0;
            let mut errors = Vec::new();
            for file in files {
                match self.vault.import_markdown(&file) {
                    Ok(i) => {
                        last_title = Some(self.vault.notes[i].title.clone());
                        count += 1;
                    }
                    Err(e) => errors.push(format!("{}: {e}", file.display())),
                }
            }
            self.selected = last_title.or(old_title).and_then(|t| self.vault.find(&t));
            self.draft = self
                .selected
                .map(|i| self.vault.notes[i].body.clone())
                .unwrap_or_default();
            self.query.clear();
            self.mode = Mode::Notes;
            self.status = format!("Imported {count} Markdown file(s). Sources preserved.");
            if !errors.is_empty() {
                self.error = Some(errors.join("\n"));
            }
        }
    }
    fn export_note(&mut self) {
        let Some(i) = self.selected else {
            return;
        };
        if let Some(mut path) = rfd::FileDialog::new()
            .set_title("Export Markdown — choose a new filename")
            .set_file_name(
                self.vault.notes[i]
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            )
            .add_filter("Markdown", &["md"])
            .save_file()
        {
            if path.extension().is_none() {
                path.set_extension("md");
            }
            match atlas::write_new(&path, self.draft.as_bytes()) {
                Ok(()) => self.status = format!("Exported current draft to {}", path.display()),
                Err(e) => self.error = Some(e),
            }
        }
    }
    fn export_notebook(&mut self) {
        if !self.save() || !self.save_board() {
            return;
        }
        if let Some(parent) = rfd::FileDialog::new()
            .set_title("Choose a folder for your notebook export")
            .pick_folder()
        {
            let mut destination = parent.join("Atlas Export");
            let mut n = 2;
            while destination.exists() {
                destination = parent.join(format!("Atlas Export {n}"));
                n += 1;
            }
            match self.vault.export_notebook(&destination) {
                Ok(()) => self.status = format!("Exported notebook to {}", destination.display()),
                Err(e) => {
                    self.error = Some(format!(
                        "Export incomplete at {}. {e}",
                        destination.display()
                    ))
                }
            }
        }
    }
    fn duplicate_note(&mut self) {
        if !self.save() {
            return;
        }
        let Some(i) = self.selected else {
            return;
        };
        match self.vault.create_neighbor(i, &self.draft, "copy") {
            Ok(i) => {
                self.selected = Some(i);
                self.status = "Created an independent copy".into();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn files_menu(&mut self, ui: &mut Ui) {
        let files = icon_button(ui, Icon::Files, "Files · new, import, export and Trash");
        Popup::menu(&files).show(|ui| {
                if ui.button("New note                 ⌘N").clicked() { self.new_dialog(); ui.close(); }
                if ui.button("Import Markdown…         ⌘O").clicked() { ui.close(); self.import_files(); }
                if ui.button("Import folder + assets…").clicked() { ui.close(); self.import_folder(); }
                ui.separator();
                if ui.add_enabled(self.selected.is_some(), Button::new("Save now                 ⌘S")).clicked() {
                    self.save(); self.save_board(); ui.close();
                }
                if ui.add_enabled(self.selected.is_some(), Button::new("Export Markdown…        ⇧⌘S")).clicked() { ui.close(); self.export_note(); }
                if ui.button("Export notebook + attachments…").clicked() { ui.close(); self.export_notebook(); }
                if ui.add_enabled(self.selected.is_some(), Button::new("Duplicate note")).clicked() { self.duplicate_note(); ui.close(); }
                ui.separator();
                if ui.add_enabled(self.selected.is_some(),Button::new("Rename note…")).clicked() {
                    self.rename_title=self.selected.map(|i|self.vault.notes[i].path.file_stem().unwrap_or_default().to_string_lossy().into_owned()).unwrap_or_default();
                    self.rename_dialog=true; ui.close();
                }
                if ui.add_enabled(self.selected.is_some(),Button::new("Delete note · move to Trash")).clicked() { self.move_to_trash(); ui.close(); }
                if ui.button("Trash · restore notes…").clicked() {self.trash_dialog=true;ui.close();}
                ui.separator();
                ui.label(R::new("Folder imports keep local assets and folders.\nSingle-file imports copy Markdown text only.").size(11.).color(MUTED));
            });
    }
    fn appearance_menu(&mut self, ui: &mut Ui) {
        let before = self.appearance.clone();
        ui.menu_button("Paper & ink", |ui| {
            ui.selectable_value(&mut self.fountain, true, "Cursive fountain pen");
            ui.selectable_value(&mut self.fountain, false, "Typeset");
            ui.separator();
            ui.label(R::new("PAPER & INK").size(10.).color(MUTED));
            for kind in washi::PaperKind::ALL {
                if ui
                    .selectable_label(
                        self.appearance.kind == kind,
                        R::new(kind.name()).color(kind.ink()),
                    )
                    .clicked()
                {
                    self.appearance.kind = kind;
                }
            }
            ui.separator();
            ui.label("Paper texture");
            ui.add(
                Slider::new(&mut self.appearance.grain, 0.0..=1.0)
                    .text("Fiber")
                    .show_value(false),
            );
            ui.label(
                R::new("Smooth                     Handmade")
                    .size(10.)
                    .color(MUTED),
            );
        });
        self.appearance.fountain = self.fountain;
        if self.appearance != before {
            if self.appearance.grain != before.grain {
                self.paper_texture = washi::texture(ui.ctx(), self.appearance.grain);
            }
            let directory = self.vault.root.join(".atlas");
            let result = std::fs::create_dir_all(&directory).and_then(|()| {
                let bytes = serde_json::to_vec_pretty(&self.appearance)?;
                let temp = directory.join("appearance.json.tmp");
                std::fs::write(&temp, bytes)?;
                std::fs::rename(temp, directory.join("appearance.json"))
            });
            if let Err(e) = result {
                self.error = Some(format!("Could not save appearance: {e}"));
            }
        }
    }
    fn save(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        if let Some(i) = self.selected {
            match self.vault.save(i, &self.draft) {
                Ok(()) => {
                    self.dirty = false;
                    self.status = "All changes saved".into();
                    true
                }
                Err(e) => {
                    self.status = "Save failed — draft preserved".into();
                    self.error = Some(e);
                    false
                }
            }
        } else {
            true
        }
    }
    fn save_board(&mut self) -> bool {
        if !self.board_dirty {
            return true;
        }
        match self.vault.save_board() {
            Ok(()) => {
                self.board_dirty = false;
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    fn select(&mut self, i: usize) {
        if self.selected == Some(i) {
            return;
        }
        if self.save() {
            self.selected = Some(i);
            self.draft = self.vault.notes[i].body.clone();
            self.editing = false;
        }
    }
    fn touch(&mut self) {
        self.dirty = true;
        self.changed = Instant::now();
        self.status = "Saving...".into();
    }
    fn touch_board(&mut self) {
        self.board_dirty = true;
        self.board_changed = Instant::now();
    }
    fn new_dialog(&mut self) {
        self.new_note = true;
        self.new_title.clear();
        self.new_note_error = None;
        self.focus_new = true;
    }
    fn navigate(&mut self, title: &str) {
        if let Some(i) = self.vault.resolve(title, self.selected) {
            self.select(i);
            self.mode = Mode::Notes;
        } else {
            self.new_title = title.into();
            self.new_note = true;
            self.focus_new = true;
        }
    }
    fn open_target(&mut self, target: &str) {
        if let Some(title) = target.strip_prefix("atlas:") {
            self.navigate(&title.replace("%20", " "));
            return;
        }
        if target.starts_with("https://") || target.starts_with("http://") {
            if let Err(e) = std::process::Command::new("/usr/bin/open")
                .arg(target)
                .spawn()
            {
                self.error = Some(e.to_string())
            }
            return;
        }
        if let Some(i) = self.vault.resolve(target, self.selected) {
            self.select(i);
            self.mode = Mode::Notes;
            return;
        }
        let local = atlas::files::decode_path(target.split('#').next().unwrap_or(target));
        let parent = self
            .selected
            .and_then(|i| self.vault.notes[i].path.parent())
            .unwrap_or(&self.vault.root);
        match parent.join(&local).canonicalize() {
            Ok(p) if p.starts_with(&self.vault.root) => {
                if let Err(e) = std::process::Command::new("/usr/bin/open").arg(p).spawn() {
                    self.error = Some(e.to_string())
                }
            }
            _ => {
                self.error =
                    Some("That local attachment could not be found inside this vault".into())
            }
        }
    }
    fn pin(&mut self) {
        if !self.save() {
            return;
        }
        if let Some(i) = self.selected {
            let title = self.vault.notes[i].title.clone();
            let added = self.vault.board.pin(&title);
            self.vault.board.reveal(&title);
            self.touch_board();
            self.mode = Mode::Canvas;
            self.status = if added {
                "Pinned note to canvas"
            } else {
                "Note already pinned · brought into view"
            }
            .into();
        }
    }
    fn reload(&mut self) {
        if !self.save() {
            return;
        }
        let title = self.selected.map(|i| self.vault.notes[i].title.clone());
        match self.vault.reload() {
            Ok(()) => {
                self.selected =
                    title
                        .and_then(|t| self.vault.find(&t))
                        .or(if self.vault.notes.is_empty() {
                            None
                        } else {
                            Some(0)
                        });
                self.draft = self
                    .selected
                    .map(|i| self.vault.notes[i].body.clone())
                    .unwrap_or_default();
                self.status = "Notes refreshed from disk".into();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn attach(&mut self) {
        if self.selected.is_none() {
            return;
        }
        if let Some(file) = rfd::FileDialog::new()
            .set_title("Attach a file to this note")
            .pick_file()
        {
            match self.vault.attach(&file) {
                Ok(mut link) => {
                    if let Some(i) = self.selected {
                        let parent = self.vault.notes[i]
                            .path
                            .parent()
                            .unwrap_or(&self.vault.root);
                        let relative = atlas::files::relative_path(
                            parent,
                            &self.vault.root.join("attachments"),
                        );
                        link = link.replace(
                            "](attachments/",
                            &format!("]({}/", atlas::files::encode_path(&relative)),
                        );
                    }
                    self.draft.push_str(&format!("\n\n{link}\n"));
                    self.touch();
                }
                Err(e) => self.error = Some(e),
            }
        }
    }
    fn sidebar(&mut self, ui: &mut Ui) {
        Panel::left("library")
            .exact_size(244.)
            .resizable(false)
            .frame(Frame::new().fill(PANEL).inner_margin(22))
            .show(ui, |ui| {
                washi::paper(ui.painter(), ui.max_rect().expand(22.), &self.paper_texture);
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(38., 42.), Sense::hover());
                    washi::enso(ui.painter(), r.center(), 14., INK);
                    ui.label(R::new("atlas").font(washi::serif(38.)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, Icon::Sidebar, "Hide notes sidebar").clicked() {
                            self.panels.toggle_sidebar();
                        }
                    });
                });
                ui.label(
                    R::new("T H E   W A S H I   E D I T I O N")
                        .size(8.)
                        .color(MUTED),
                );
                ui.add_space(28.);
                if ui
                    .add_sized(
                        [200., 38.],
                        Button::new(R::new("+   New note").color(PAPER)).fill(INK),
                    )
                    .on_hover_text("Command-N")
                    .clicked()
                {
                    self.new_dialog()
                }
                ui.add_space(12.);
                let response = ui.add(
                    TextEdit::singleline(&mut self.query)
                        .hint_text("Search notes or #tags")
                        .desired_width(200.)
                        .margin(vec2(10., 9.)),
                );
                if self.search_focus {
                    response.request_focus();
                    self.search_focus = false;
                }
                ui.add_space(18.);
                let results = self.vault.search(&self.query);
                ui.label(
                    R::new(format!("YOUR NOTES   {}", results.len()))
                        .size(10.)
                        .color(MUTED),
                );
                ui.add_space(8.);
                let mut note_action = None;
                ScrollArea::vertical()
                    .id_salt("note-list")
                    .max_height((ui.available_height() - 110.).max(100.))
                    .show(ui, |ui| {
                        for i in results {
                            let n = &self.vault.notes[i];
                            let title = n.title.clone();
                            let subtitle = tags(&n.body)
                                .first()
                                .map(|t| format!("#{t}"))
                                .unwrap_or_else(|| "Note".into());
                            let selected = self.selected == Some(i);
                            let frame = Frame::new()
                                .fill(if selected {
                                    C::from_rgb(223, 217, 193)
                                } else {
                                    C::TRANSPARENT
                                })
                                .corner_radius(7)
                                .inner_margin(10);
                            let resp = frame
                                .show(ui, |ui| {
                                    ui.set_min_width(176.);
                                    ui.label(R::new(&title).font(washi::serif(21.)));
                                    ui.label(R::new(subtitle).size(11.).color(MUTED));
                                })
                                .response;
                            let row = ui.interact(
                                resp.rect,
                                Id::new(("note", self.vault.notes[i].path.clone())),
                                Sense::click(),
                            );
                            if row.clicked() {
                                self.select(i);
                            }
                            row.context_menu(|ui| {
                                ui.label(R::new(&title).strong());
                                ui.separator();
                                for (label, action) in [
                                    ("Open", "open"),
                                    ("Rename…", "rename"),
                                    ("Duplicate", "duplicate"),
                                    ("Export Markdown…", "export"),
                                ] {
                                    if ui.button(label).clicked() {
                                        note_action = Some((i, action));
                                        ui.close();
                                    }
                                }
                                ui.separator();
                                if ui.button("Move to Trash").clicked() {
                                    note_action = Some((i, "trash"));
                                    ui.close();
                                }
                            });
                            ui.add_space(3.);
                        }
                    });
                if let Some((i, action)) = note_action {
                    self.select(i);
                    if self.selected == Some(i) {
                        match action {
                            "rename" => {
                                self.rename_title = self.vault.notes[i]
                                    .path
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned();
                                self.rename_dialog = true;
                            }
                            "duplicate" => self.duplicate_note(),
                            "export" => self.export_note(),
                            "trash" => self.move_to_trash(),
                            _ => self.mode = Mode::Notes,
                        }
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(
                        R::new("A quiet place. Entirely yours.")
                            .size(11.)
                            .color(MUTED),
                    );
                    ui.horizontal(|ui| {
                        if icon_button(ui, Icon::Folder, "Open notebook folder in Finder").clicked()
                        {
                            let _ = std::process::Command::new("/usr/bin/open")
                                .arg(&self.vault.root)
                                .spawn();
                        }
                        if icon_button(
                            ui,
                            Icon::Refresh,
                            "Refresh notes from disk · pick up changes made outside Atlas",
                        )
                        .clicked()
                        {
                            self.reload()
                        }
                    });
                    ui.label(R::new("YOUR PAPER GARDEN").size(10.).color(GREEN));
                });
            });
    }
    fn inspector(&mut self, ui: &mut Ui) {
        Panel::right("inspector")
            .exact_size(235.)
            .resizable(false)
            .frame(Frame::new().fill(PAPER).inner_margin(22))
            .show(ui, |ui| {
                washi::paper(ui.painter(), ui.max_rect().expand(22.), &self.paper_texture);
                ui.add_space(14.);
                ui.label(R::new("THREADS OF THOUGHT").size(10.).color(MUTED));
                ui.add_space(15.);
                if let Some(i) = self.selected {
                    let title = self.vault.notes[i].title.clone();
                    let links = wiki_links(&self.draft);
                    let backlinks = self.vault.backlinks(&title);
                    ui.label(R::new("From this note").strong());
                    ui.add_space(4.);
                    if links.is_empty() {
                        ui.label(
                            R::new("Link an idea using [[Note title]].")
                                .size(12.)
                                .color(MUTED),
                        );
                    }
                    for link in links {
                        let exists = self.vault.resolve(&link, self.selected).is_some();
                        if ui
                            .add(
                                Button::new(
                                    R::new(format!("{} {}", if exists { "↗" } else { "+" }, link))
                                        .color(GREEN),
                                )
                                .wrap()
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.navigate(&link)
                        }
                    }
                    ui.add_space(10.);
                    ui.separator();
                    ui.add_space(14.);
                    ui.label(R::new("Mentioned in").strong());
                    if backlinks.is_empty() {
                        ui.label(
                            R::new("Other notes that link here will appear below.")
                                .size(12.)
                                .color(MUTED),
                        );
                    }
                    for j in backlinks {
                        let name = self.vault.notes[j].title.clone();
                        if ui
                            .add(Button::new(R::new(name).color(GREEN)).frame(false).wrap())
                            .clicked()
                        {
                            self.select(j)
                        }
                    }
                    ui.add_space(25.);
                    ui.separator();
                    ui.add_space(14.);
                    ui.label(R::new("ON THIS PAGE").size(10.).color(MUTED));
                    for tag in tags(&self.draft) {
                        if ui.small_button(format!("#{tag}")).clicked() {
                            self.query = format!("#{tag}")
                        }
                    }
                    ui.add_space(20.);
                    ui.label(
                        R::new(format!(
                            "{} words  ·  {} links",
                            self.draft.split_whitespace().count(),
                            wiki_links(&self.draft).len()
                        ))
                        .size(11.)
                        .color(MUTED),
                    );
                    ui.add_space(20.);
                    ui.menu_button("Insert a link", |ui| {
                        let names: Vec<_> =
                            self.vault.notes.iter().map(|n| n.title.clone()).collect();
                        for name in names {
                            if name != title && ui.button(&name).clicked() {
                                self.draft.push_str(&format!("\n[[{name}]]"));
                                self.touch();
                                ui.close();
                            }
                        }
                    });
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    washi::landscape(ui);
                    ui.label(
                        R::new("Leave room for wonder.")
                            .font(washi::pen(21.))
                            .color(MUTED),
                    );
                });
            });
    }
    fn toolbar(&mut self, ui: &mut Ui) {
        use atlas::editing::Format;
        let ctx = ui.ctx().clone();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 38.), Sense::hover());
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(Rect::from_min_size(
                    rect.min,
                    vec2(if !self.panels.sidebar { 240. } else { 115. }, 38.),
                ))
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                if !self.panels.sidebar {
                    if icon_button(ui, Icon::Sidebar, "Show notes sidebar").clicked() {
                        self.panels.sidebar = true;
                    }
                    let (r, _) = ui.allocate_exact_size(vec2(20., 26.), Sense::hover());
                    washi::enso(ui.painter(), r.center(), 8., INK);
                    ui.label(R::new("atlas").font(washi::serif(25.)));
                }
                self.files_menu(ui);
                if icon_toggle(
                    ui,
                    Icon::Canvas,
                    if self.mode == Mode::Canvas {
                        "Return to notebook"
                    } else {
                        "Open canvas"
                    },
                    self.mode == Mode::Canvas,
                )
                .clicked()
                    && self.save()
                {
                    self.mode = if self.mode == Mode::Canvas {
                        Mode::Notes
                    } else {
                        Mode::Canvas
                    };
                }
                ui.add_enabled_ui(self.selected.is_some(), |ui| {
                    if icon_button(ui, Icon::Pin, "Pin selected note to canvas").clicked() {
                        self.pin();
                    }
                });
            },
        );
        let capsule = Rect::from_center_size(rect.center(), vec2(200., 36.));
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(capsule)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                Frame::new()
                    .fill(self.appearance.kind.sheet())
                    .stroke(Stroke::new(0.7, LINE))
                    .corner_radius(18)
                    .inner_margin(Margin::symmetric(10, 5))
                    .show(ui, |ui| {
                        ui.add_enabled_ui(
                            self.selected.is_some() && self.mode == Mode::Notes,
                            |ui| {
                                ui.spacing_mut().item_spacing.x = 6.;
                                let mut chosen = None;
                                let styles = icon_button(ui, Icon::TextStyle, "Text styles");
                                Popup::menu(&styles).show(|ui| {
                                    for (label, format) in [
                                        ("Heading", Format::Heading),
                                        ("Bold     ⌘B", Format::Bold),
                                        ("Italic     ⌘I", Format::Italic),
                                        ("Code", Format::Code),
                                    ] {
                                        if ui.button(label).clicked() {
                                            chosen = Some(format);
                                            ui.close();
                                        }
                                    }
                                });
                                ui.separator();
                                let lists = icon_button(
                                    ui,
                                    Icon::List,
                                    "Lists · bullets, numbering, checklists",
                                );
                                Popup::menu(&lists).show(|ui| {
                                    for (label, format) in [
                                        ("Bulleted list", Format::Bullet),
                                        ("Numbered list", Format::Numbered),
                                        ("Checklist", Format::Task),
                                    ] {
                                        if ui.button(label).clicked() {
                                            chosen = Some(format);
                                            ui.close();
                                        }
                                    }
                                });
                                if icon_button(ui, Icon::Checklist, "Checklist").clicked() {
                                    chosen = Some(Format::Task);
                                }
                                if icon_button(ui, Icon::Link, "Insert Markdown link").clicked() {
                                    chosen = Some(Format::Link);
                                }
                                if icon_button(ui, Icon::Attachment, "Attach a file").clicked() {
                                    self.attach();
                                }
                                if let Some(format) = chosen {
                                    self.editing = true;
                                    self.format_text(&ctx, format);
                                }
                            },
                        );
                    });
            },
        );
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(Rect::from_min_max(
                    pos2(rect.right() - 145., rect.top()),
                    rect.right_bottom(),
                ))
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                let more = icon_button(ui, Icon::More, "Appearance and view");
                Popup::menu(&more).show(|ui| {
                    self.appearance_menu(ui);
                });
                ui.add_enabled_ui(self.selected.is_some(), |ui| {
                    if icon_button(
                        ui,
                        Icon::Trash,
                        "Move note to Trash · restore from Files → Trash",
                    )
                    .clicked()
                    {
                        self.move_to_trash();
                    }
                });
                ui.separator();
                if icon_toggle(
                    ui,
                    if self.panels.focused() {
                        Icon::Unfocus
                    } else {
                        Icon::Focus
                    },
                    if self.panels.focused() {
                        "Leave focus · ⌘⇧F"
                    } else {
                        "Focus on writing · ⌘⇧F"
                    },
                    self.panels.focused(),
                )
                .clicked()
                {
                    self.panels.toggle_focus();
                }
                ui.add_enabled_ui(self.selected.is_some() && self.mode == Mode::Notes, |ui| {
                    if icon_toggle(
                        ui,
                        if self.editing { Icon::Done } else { Icon::Edit },
                        if self.editing {
                            "Done · show rendered page"
                        } else {
                            "Edit Markdown"
                        },
                        self.editing,
                    )
                    .clicked()
                    {
                        self.editing = !self.editing;
                    }
                });
            },
        );
    }
    fn notebook(&mut self, ui: &mut Ui) {
        let Some(i) = self.selected else {
            if self.panels.focused() && ui.button("Leave focus").clicked() {
                self.panels.toggle_focus();
            }
            ui.heading("An empty page. A fresh beginning.");
            if ui.button("New note").clicked() {
                self.new_dialog()
            }
            return;
        };
        let title = self.vault.notes[i].title.clone();
        ui.add_space(6.);
        let width = ui.available_width().min(900.);
        let inset = ((ui.available_width() - width) / 2.).max(0.);
        let mut paper_right = ui.max_rect().right();
        let min_page_height = ui.available_height();
        let document = ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt(("document", title))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(inset);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        let page = ui.available_rect_before_wrap();
                        paper_right = page.left() + width;
                        // Reserve a paint slot behind the content, then size it from the
                        // completed layout so the sheet grows in the same frame as typing.
                        let background = ui.painter().add(Shape::Noop);
                        let content =
                            Frame::new()
                                .inner_margin(Margin::symmetric(38, 32))
                                .show(ui, |ui| {
                                    ui.set_width((width - 76.).max(200.));
                                    ui.set_min_height((min_page_height - 64.).max(0.));
                                    ui.label(
                                        R::new(&self.vault.notes[i].title).size(10.).color(MUTED),
                                    );
                                    ui.add_space(10.);
                                    ui.spacing_mut().extra_text_line_spacing = 5.;
                                    if self.editing {
                                        self.editor(ui);
                                    } else {
                                        let source = wiki_markdown(&self.draft);
                                        if let Some(target) = markdown(
                                            ui,
                                            &source,
                                            self.fountain,
                                            self.appearance.kind.ink(),
                                        ) {
                                            self.open_target(&target)
                                        }
                                    }
                                });
                        let sheet = Rect::from_min_size(
                            page.min,
                            vec2(width, content.response.rect.height().max(min_page_height)),
                        );
                        ui.painter().set(
                            background,
                            washi::sheet_shape(
                                sheet,
                                ui.clip_rect(),
                                &self.paper_texture,
                                self.appearance.kind.sheet(),
                            ),
                        );
                    });
                });
            });
        // Painted outside the scrolling content: a quiet seal in the paper margin.
        let seal = Rect::from_min_size(
            pos2(
                paper_right.min(document.inner_rect.right()) - 36.,
                document.inner_rect.bottom() - 48.,
            ),
            vec2(24., 30.),
        );
        washi::stamp(&ui.painter().with_clip_rect(document.inner_rect), seal);
    }
    fn canvas(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                R::new("Canvas · drag cards or the background · double-click to open")
                    .size(11.)
                    .color(MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, Icon::Home, "Bring selected card into view").clicked() {
                    if let Some(i) = self.selected {
                        self.vault.board.reveal(&self.vault.notes[i].title);
                    } else {
                        self.vault.board.pan = [0., 0.];
                    }
                    self.touch_board();
                }
            });
        });
        ui.add_space(8.);
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let area = response.rect;
        painter.rect_filled(area, 2, PANEL);
        washi::paper(&painter, area, &self.paper_texture);
        let painter = painter.with_clip_rect(area.intersect(ui.clip_rect()));
        let pan = vec2(self.vault.board.pan[0], self.vault.board.pan[1]);
        // Faint graphite crosses keep the board calm while providing spatial reference.
        for x in (20..area.width() as usize).step_by(64) {
            for y in (20..area.height() as usize).step_by(64) {
                let c = area.min + vec2(x as f32, y as f32);
                painter.line_segment([c - vec2(2., 0.), c + vec2(2., 0.)], Stroke::new(0.5, LINE));
                painter.line_segment([c - vec2(0., 2.), c + vec2(0., 2.)], Stroke::new(0.5, LINE));
            }
        }
        let mut cards = Vec::new();
        for (title, pos) in &self.vault.board.positions {
            if let Some(i) = self.vault.find(title) {
                cards.push((
                    i,
                    title.clone(),
                    Rect::from_min_size(area.min + vec2(pos[0], pos[1]) + pan, vec2(248., 154.)),
                ));
            }
        }
        for (i, _, rect) in &cards {
            for link in wiki_links(&self.vault.notes[*i].body) {
                if let Some((_, _, other)) = cards
                    .iter()
                    .find(|(other_i, _, _)| self.vault.resolve(&link, Some(*i)) == Some(*other_i))
                {
                    let a = rect.center();
                    let b = other.center();
                    if a.distance(b) < 1. {
                        continue;
                    }
                    painter.line_segment([a, b], Stroke::new(1.5, C::from_rgb(145, 166, 148)));
                    let mid = a.lerp(b, 0.55);
                    let d = (b - a).normalized();
                    painter.line_segment(
                        [mid, mid - d.rot90() * 4. - d * 8.],
                        Stroke::new(1.5, GREEN),
                    );
                    painter.line_segment(
                        [mid, mid + d.rot90() * 4. - d * 8.],
                        Stroke::new(1.5, GREEN),
                    );
                }
            }
        }
        let mut any_drag = false;
        for (i, title, rect) in &cards {
            if !area.intersects(*rect) {
                continue;
            }
            let r = ui.interact(
                rect.intersect(area),
                Id::new(("card", title)),
                Sense::click_and_drag(),
            );
            r.widget_info(|| {
                WidgetInfo::labeled(WidgetType::Button, true, format!("Canvas note: {title}"))
            });
            let selected = self.selected == Some(*i);
            let painter = painter.with_clip_rect(rect.intersect(area));
            painter.rect_filled(rect.translate(vec2(0., 3.)), 8, C::from_black_alpha(10));
            painter.rect_filled(*rect, 2, PAPER);
            washi::paper(&painter, *rect, &self.paper_texture);
            painter.rect_stroke(
                *rect,
                8,
                Stroke::new(
                    if selected { 2. } else { 1. },
                    if selected { GREEN } else { LINE },
                ),
                StrokeKind::Inside,
            );
            painter.rect_filled(
                Rect::from_min_size(rect.min + vec2(16., 17.), vec2(23., 3.)),
                1,
                if selected { ACCENT } else { GREEN },
            );
            let mut card_title = title.chars().take(24).collect::<String>();
            if title.chars().count() > 24 {
                card_title.push('…');
            }
            let heading = painter.layout(card_title, washi::serif(22.), INK, 216.);
            painter.galley(rect.min + vec2(16., 32.), heading, INK);
            let excerpt = self.vault.notes[*i]
                .body
                .lines()
                .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            let excerpt = excerpt
                .replace("**", "")
                .replace("[[", "")
                .replace("]]", "");
            let mut short = excerpt.chars().take(70).collect::<String>();
            if excerpt.chars().count() > 70 {
                short.push('…')
            }
            let galley = painter.layout(short, washi::pen(17.), MUTED, 216.);
            painter.galley(rect.min + vec2(16., 61.), galley, MUTED);
            let label = tags(&self.vault.notes[*i].body)
                .first()
                .map(|t| format!("#{t}"))
                .unwrap_or_else(|| "NOTE".into());
            painter.text(
                rect.min + vec2(16., 132.),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(10.),
                GREEN,
            );
            if r.clicked() {
                self.select(*i)
            }
            if r.double_clicked() {
                self.select(*i);
                if self.selected == Some(*i) {
                    self.mode = Mode::Notes;
                }
            }
            if r.dragged() {
                any_drag = true;
                let d = ui.input(|x| x.pointer.delta());
                if let Some(pos) = self.vault.board.positions.get_mut(title) {
                    pos[0] += d.x;
                    pos[1] += d.y;
                }
                self.touch_board();
            }
            r.context_menu(|ui| {
                if ui.button("Remove from canvas").clicked() {
                    self.vault.board.positions.remove(title);
                    self.touch_board();
                    ui.close();
                }
            });
        }
        if response.dragged() && !any_drag {
            let d = ui.input(|x| x.pointer.delta());
            self.vault.board.pan[0] += d.x;
            self.vault.board.pan[1] += d.y;
            self.touch_board();
        }
        if cards.is_empty() {
            painter.text(
                area.center(),
                Align2::CENTER_CENTER,
                "Pin a note to start connecting ideas.",
                FontId::proportional(18.),
                MUTED,
            );
        }
    }
}

impl eframe::App for Atlas {
    fn logic(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        if self.dirty && self.changed.elapsed() > Duration::from_millis(750) && self.error.is_none()
        {
            self.save();
        }
        if self.board_dirty
            && self.board_changed.elapsed() > Duration::from_millis(750)
            && self.error.is_none()
        {
            self.save_board();
        }
        if ctx.input(|i| i.viewport().close_requested()) && (!self.save() || !self.save_board()) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }
        ctx.request_repaint_after(Duration::from_millis(300));
    }
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::N)) {
            self.new_dialog()
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::O)) {
            self.import_files();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::S)) {
            self.export_note();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::S)) {
            self.save();
            self.save_board();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::F)) {
            self.panels.toggle_focus();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::K)) {
            self.panels.sidebar = true;
            self.search_focus = true;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Q))
            && self.save()
            && self.save_board()
        {
            ctx.send_viewport_cmd(ViewportCommand::Close)
        }
        Panel::bottom("status")
            .frame(
                Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(20, 7)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(R::new("●").color(GREEN).size(10.));
                    ui.label(R::new(&self.status).size(11.).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            R::new("⌘N  New note    ⌘K  Find    ⌘⇧F  Focus    ⌘S  Save")
                                .size(11.)
                                .color(MUTED),
                        );
                    });
                });
            });
        if self.panels.sidebar {
            self.sidebar(ui);
        }
        if self.panels.inspector {
            self.inspector(ui);
        }
        CentralPanel::default()
            .frame(Frame::new().fill(PAPER).inner_margin(30))
            .show(ui, |ui| {
                washi::paper(ui.painter(), ui.max_rect().expand(30.), &self.paper_texture);
                self.toolbar(ui);
                ui.add_space(8.);
                if self.mode == Mode::Notes {
                    self.notebook(ui)
                } else {
                    self.canvas(ui)
                }
            });
        if self.new_note {
            let mut create = false;
            let mut cancel = false;
            let ink = self.appearance.kind.ink();
            let modal = Modal::new(Id::new("new_note_modal"))
                .backdrop_color(C::from_black_alpha(55))
                .frame(
                    Frame::new()
                        .fill(self.appearance.kind.sheet())
                        .stroke(Stroke::new(1., LINE))
                        .corner_radius(8)
                        .inner_margin(28),
                )
                .show(&ctx, |ui| {
                    ui.set_width(420.);
                    let texture = ui.painter().add(Shape::Noop);
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(26., 32.), Sense::hover());
                        washi::enso(ui.painter(), r.center(), 10., ink);
                        ui.label(R::new("New note").font(washi::serif(32.)).color(ink));
                    });
                    ui.label(R::new("A little space for a new thought.").color(MUTED));
                    ui.add_space(18.);
                    let label = ui.label(R::new("Note title").size(12.).color(MUTED));
                    let r = ui
                        .add(
                            TextEdit::singleline(&mut self.new_title)
                                .font(if self.appearance.fountain {
                                    washi::script(32.)
                                } else {
                                    washi::serif(26.)
                                })
                                .text_color(ink)
                                .margin(vec2(12., 10.))
                                .desired_width(f32::INFINITY)
                                .hint_text("Name your note"),
                        )
                        .labelled_by(label.id);
                    if self.focus_new {
                        r.request_focus();
                        self.focus_new = false;
                    }
                    if r.changed() {
                        self.new_note_error = None;
                    }
                    let has_title = !self.new_title.trim().is_empty();
                    if has_title && r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        create = true;
                    }
                    if let Some(error) = &self.new_note_error {
                        ui.add_space(6.);
                        ui.label(R::new(error).color(ACCENT).size(12.));
                    }
                    ui.add_space(20.);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        create |= ui
                            .add_enabled(
                                has_title,
                                Button::new(R::new("Create note").color(PAPER))
                                    .fill(ink)
                                    .min_size(vec2(122., 36.)),
                            )
                            .clicked();
                        cancel |= ui
                            .add(Button::new("Cancel").min_size(vec2(82., 36.)))
                            .clicked();
                    });
                    let paper = ui.min_rect().expand(20.);
                    ui.painter().set(
                        texture,
                        Shape::image(
                            self.paper_texture.id(),
                            paper,
                            Rect::from_min_max(
                                pos2(paper.left() / 768., paper.top() / 768.),
                                pos2(paper.right() / 768., paper.bottom() / 768.),
                            ),
                            C::WHITE,
                        ),
                    );
                });
            cancel |= modal.should_close();
            if create && self.save() {
                let title = self.new_title.trim().to_owned();
                match self.vault.create(&title, &format!("# {title}\n\n")) {
                    Ok(i) => {
                        self.selected = Some(i);
                        self.draft = self.vault.notes[i].body.clone();
                        self.dirty = false;
                        self.status = "New note saved".into();
                        self.new_note = false;
                        self.focus_editor = true;
                        self.mode = Mode::Notes;
                        self.editing = true;
                    }
                    Err(e) => self.new_note_error = Some(e),
                }
            }
            if cancel {
                self.new_note = false;
            }
        }
        if self.rename_dialog {
            let mut apply = false;
            let mut cancel = false;
            Window::new("Rename note").collapsible(false).anchor(Align2::CENTER_CENTER,[0.,0.]).show(&ctx,|ui| {
                ui.label("Rename the file in its current folder. Wiki links and inline Markdown links update automatically.");
                ui.label(R::new("The heading inside the note stays as written. Reference-style links are not rewritten.").small().color(MUTED));
                ui.text_edit_singleline(&mut self.rename_title);
                ui.horizontal(|ui|{ apply=ui.button("Rename").clicked();cancel=ui.button("Cancel").clicked(); });
            });
            if apply
                && self.save()
                && self.save_board()
                && let Some(i) = self.selected
            {
                match self.vault.rename_note(i, &self.rename_title) {
                    Ok(i) => {
                        self.select_fresh(Some(i));
                        self.rename_dialog = false;
                        self.status = "Renamed note and updated links".into();
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            if cancel {
                self.rename_dialog = false;
            }
        }
        if self.trash_dialog {
            let mut restore = None;
            let mut close = false;
            Window::new("Trash")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, [0., 0.])
                .show(&ctx, |ui| {
                    ui.label("Notes stay here until restored. Existing files are never replaced.");
                    match self.vault.trashed_notes() {
                        Ok(notes) => {
                            if notes.is_empty() {
                                ui.label("Trash is empty.");
                            }
                            ScrollArea::vertical().max_height(320.).show(ui, |ui| {
                                for note in notes {
                                    ui.horizontal(|ui| {
                                        ui.label(&note.title);
                                        if ui.button("Restore").clicked() {
                                            restore = Some(note.id);
                                        }
                                    });
                                }
                            });
                        }
                        Err(e) => {
                            ui.label(e);
                        }
                    }
                    close = ui.button("Done").clicked();
                });
            if let Some(id) = restore
                && self.save()
            {
                match self.vault.restore_note(&id) {
                    Ok(i) => {
                        self.select_fresh(Some(i));
                        self.status = "Restored note".into();
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            if close {
                self.trash_dialog = false;
            }
        }
        if let Some(error) = self.error.clone() {
            Window::new("Your work needs attention")
                .collapsible(false)
                .anchor(Align2::CENTER_CENTER, [0., 0.])
                .show(&ctx, |ui| {
                    ui.set_max_width(470.);
                    ui.label(&error);
                    ui.add_space(12.);
                    ui.horizontal(|ui| {
                        if ui.button("Dismiss").clicked() {
                            self.error = None;
                            self.changed = Instant::now();
                        }
                        if self.dirty && ui.button("Save draft as new note").clicked() {
                            let result = if let Some(i) = self.selected {
                                self.vault.create_neighbor(i, &self.draft, "recovered")
                            } else {
                                self.vault.create_copy("Recovered note", &self.draft)
                            };
                            match result {
                                Ok(i) => {
                                    self.selected = Some(i);
                                    self.dirty = false;
                                    self.error = None;
                                    self.status = "Draft saved as a new note".into();
                                }
                                Err(e) => self.error = Some(e),
                            }
                        }
                    });
                });
        }
        self.frames += 1;
        if self.frames == 8 && self.screenshot.is_some() {
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(Default::default()));
        }
        let images: Vec<_> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
                .collect()
        });
        for image in images {
            if let Some(path) = self.screenshot.take() {
                match save_png(&path, &image) {
                    Ok(()) => {
                        if self.quit_after {
                            ctx.send_viewport_cmd(ViewportCommand::Close)
                        }
                    }
                    Err(e) => self.error = Some(e),
                }
            }
        }
        if self.screenshot.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}

fn wiki_markdown(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some((before, tail)) = rest.split_once("[[") {
        out.push_str(before);
        if let Some((inside, after)) = tail.split_once("]]") {
            let (target, label) = inside.split_once('|').unwrap_or((inside, inside));
            let target = target.split('#').next().unwrap_or(target);
            out.push_str(&format!(
                "[{label}](atlas:{})",
                target.trim().replace(' ', "%20")
            ));
            rest = after
        } else {
            out.push_str("[[");
            rest = tail;
            break;
        }
    }
    out.push_str(rest);
    out
}
#[derive(Default)]
struct Span {
    text: String,
    bold: bool,
    italic: bool,
    code: bool,
    task: Option<bool>,
    link: Option<String>,
}
fn markdown(ui: &mut Ui, text: &str, fountain: bool, ink: C) -> Option<String> {
    let mut clicked = None;
    let mut spans = Vec::new();
    let (mut bold, mut italic, mut link) = (false, false, None);
    let mut heading = 0;
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut codeblock = false;
    fn flush(
        ui: &mut Ui,
        spans: &mut Vec<Span>,
        heading: u8,
        clicked: &mut Option<String>,
        fountain: bool,
        ink: C,
    ) {
        if spans.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.;
            for s in spans.drain(..) {
                if let Some(done) = s.task {
                    let (rect, _) = ui.allocate_exact_size(vec2(27., 30.), Sense::hover());
                    let box_rect = Rect::from_center_size(rect.center(), vec2(15., 15.));
                    ui.painter().rect_stroke(
                        box_rect,
                        2.,
                        Stroke::new(1.3, ink),
                        StrokeKind::Inside,
                    );
                    if done {
                        ui.painter().line_segment(
                            [
                                box_rect.left_center() + vec2(3., 0.),
                                box_rect.center() + vec2(-1., 4.),
                            ],
                            Stroke::new(1.5, ink),
                        );
                        ui.painter().line_segment(
                            [
                                box_rect.center() + vec2(-1., 4.),
                                box_rect.right_top() + vec2(-2., 3.),
                            ],
                            Stroke::new(1.5, ink),
                        );
                    }
                    continue;
                }
                let font = if fountain {
                    washi::script(match heading {
                        1 => 46.,
                        2 => 38.,
                        3 => 34.,
                        _ => 30.,
                    })
                } else {
                    washi::serif(match heading {
                        1 => 43.,
                        2 => 28.,
                        3 => 25.,
                        _ => 23.,
                    })
                };
                let mut r = R::new(s.text).font(font).color(ink);
                if s.bold {
                    r = r.strong()
                }
                if s.italic {
                    r = r.italics()
                }
                if s.code {
                    r = r.font(FontId::monospace(14.)).background_color(PANEL)
                }
                if let Some(link) = s.link {
                    if ui.link(r.color(GREEN)).clicked() {
                        *clicked = Some(link)
                    }
                } else {
                    ui.add(Label::new(r).wrap());
                }
            }
        });
        ui.add_space(if heading > 0 { 18. } else { 14. });
    }
    for event in Parser::new_ext(
        text,
        Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH,
    ) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = level as u8;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(ui, &mut spans, heading, &mut clicked, fountain, ink);
                heading = 0;
            }
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) | Event::End(TagEnd::Image) => link = None,
            Event::Start(Tag::List(start)) => {
                flush(ui, &mut spans, 0, &mut clicked, fountain, ink);
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
                ui.add_space(8.);
            }
            Event::Start(Tag::Item) => {
                let indent = "    ".repeat(lists.len().saturating_sub(1));
                let marker = if let Some(Some(number)) = lists.last_mut() {
                    let marker = format!("{number}.  ");
                    *number += 1;
                    marker
                } else {
                    "•  ".into()
                };
                spans.push(Span {
                    text: format!("{indent}{marker}"),
                    ..Default::default()
                });
            }
            Event::End(TagEnd::Item) => flush(ui, &mut spans, 0, &mut clicked, fountain, ink),
            Event::Start(Tag::CodeBlock(_)) => codeblock = true,
            Event::End(TagEnd::CodeBlock) => {
                flush(ui, &mut spans, 0, &mut clicked, fountain, ink);
                codeblock = false;
            }
            Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) => spans.push(Span {
                text: t.to_string(),
                bold,
                italic,
                link: link.clone(),
                code: codeblock,
                task: None,
            }),
            Event::Code(t) => spans.push(Span {
                text: t.to_string(),
                code: true,
                ..Default::default()
            }),
            Event::SoftBreak => spans.push(Span {
                text: " ".into(),
                ..Default::default()
            }),
            Event::HardBreak => flush(ui, &mut spans, heading, &mut clicked, fountain, ink),
            Event::End(TagEnd::Paragraph) => {
                flush(ui, &mut spans, heading, &mut clicked, fountain, ink)
            }
            Event::Rule => {
                ui.separator();
            }
            Event::TaskListMarker(done) => {
                // Replace the item bullet, rather than displaying both markers.
                if let Some(marker) = spans.last_mut() {
                    marker.text.clear();
                    marker.task = Some(done);
                }
            }
            _ => {}
        }
    }
    flush(ui, &mut spans, heading, &mut clicked, fountain, ink);
    clicked
}
#[derive(Clone, Copy)]
enum Icon {
    Sidebar,
    Trash,
    Link,
    Attachment,
    List,
    Checklist,
    TextStyle,
    More,
    Edit,
    Done,
    Focus,
    Unfocus,
    Files,
    Folder,
    Refresh,
    Canvas,
    Pin,
    Home,
}
fn icon_button(ui: &mut Ui, icon: Icon, label: &str) -> Response {
    icon_control(ui, icon, label, None)
}
fn icon_toggle(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    icon_control(ui, icon, label, Some(selected))
}
fn icon_control(ui: &mut Ui, icon: Icon, label: &str, selected: Option<bool>) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(28., 26.), Sense::click());
    response.widget_info(|| match selected {
        Some(value) => {
            WidgetInfo::selected(WidgetType::SelectableLabel, ui.is_enabled(), value, label)
        }
        None => WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label),
    });
    let visuals = ui.style().interact(&response);
    if selected == Some(true) {
        ui.painter().rect_filled(rect, 4, PANEL);
        ui.painter().line_segment(
            [
                rect.left_bottom() + vec2(6., -1.),
                rect.right_bottom() + vec2(-6., -1.),
            ],
            Stroke::new(2., GREEN),
        );
    } else if response.hovered() || response.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect, 4, visuals.bg_fill);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(1.),
            4,
            Stroke::new(1.5, GREEN),
            StrokeKind::Outside,
        );
    }
    let p = ui.painter();
    let c = rect.center();
    let stroke = Stroke::new(1.4, visuals.fg_stroke.color);
    let line = |a: [f32; 2], b: [f32; 2]| {
        p.line_segment([c + vec2(a[0], a[1]), c + vec2(b[0], b[1])], stroke);
    };
    match icon {
        Icon::Canvas => {
            for (x, y, w, h) in [(-8., -7., 8., 6.), (3., -3., 7., 10.), (-8., 3., 8., 5.)] {
                p.rect_stroke(
                    Rect::from_min_size(c + vec2(x, y), vec2(w, h)),
                    1,
                    stroke,
                    StrokeKind::Inside,
                );
            }
        }
        Icon::Pin => {
            line([-4., -8.], [4., -8.]);
            line([-3., -8.], [-3., -1.]);
            line([3., -8.], [3., -1.]);
            line([-3., -1.], [-6., 3.]);
            line([3., -1.], [6., 3.]);
            line([-6., 3.], [6., 3.]);
            line([0., 3.], [0., 10.]);
        }
        Icon::Home => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(12., 12.)),
                2,
                stroke,
                StrokeKind::Inside,
            );
            line([-10., 0.], [-5., 0.]);
            line([5., 0.], [10., 0.]);
            line([0., -10.], [0., -5.]);
            line([0., 5.], [0., 10.]);
        }
        Icon::Files => {
            p.add(Shape::closed_line(
                vec![
                    c + vec2(-6., -9.),
                    c + vec2(2., -9.),
                    c + vec2(7., -4.),
                    c + vec2(7., 9.),
                    c + vec2(-6., 9.),
                ],
                stroke,
            ));
            line([2., -9.], [2., -4.]);
            line([2., -4.], [7., -4.]);
            line([-3., 1.], [4., 1.]);
            line([-3., 5.], [4., 5.]);
        }
        Icon::Folder => {
            p.add(Shape::closed_line(
                vec![
                    c + vec2(-9., 7.),
                    c + vec2(-9., -7.),
                    c + vec2(-2., -7.),
                    c + vec2(1., -4.),
                    c + vec2(9., -4.),
                    c + vec2(9., 7.),
                ],
                stroke,
            ));
            line([-9., -2.], [9., -2.]);
        }
        Icon::Refresh => {
            let points = (0..=24)
                .map(|i| {
                    let angle = -0.7 + i as f32 / 24. * 5.3;
                    c + vec2(angle.cos(), angle.sin()) * 8.
                })
                .collect();
            p.add(Shape::line(points, stroke));
            line([6., -9.], [6., -4.]);
            line([6., -4.], [1., -4.]);
        }
        Icon::Edit => {
            p.add(Shape::closed_line(
                vec![
                    c + vec2(-7., 7.),
                    c + vec2(-5., 1.),
                    c + vec2(5., -9.),
                    c + vec2(9., -5.),
                    c + vec2(-1., 5.),
                ],
                stroke,
            ));
            line([3., -7.], [7., -3.]);
        }
        Icon::Done => {
            line([-7., 0.], [-2., 5.]);
            line([-2., 5.], [8., -7.]);
        }
        Icon::Focus | Icon::Unfocus => {
            for (x, y) in [(-1., -1.), (1., -1.), (-1., 1.), (1., 1.)] {
                let (outer, inner) = if matches!(icon, Icon::Focus) {
                    (8., 3.)
                } else {
                    (3., 8.)
                };
                line([x * outer, y * inner], [x * outer, y * outer]);
                line([x * outer, y * outer], [x * inner, y * outer]);
            }
        }
        Icon::List => {
            for y in [-6., 0., 6.] {
                p.circle_filled(c + vec2(-7., y), 1.4, stroke.color);
                line([-2., y], [8., y]);
            }
        }
        Icon::Checklist => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(16., 16.)),
                2,
                stroke,
                StrokeKind::Inside,
            );
            line([-4., 0.], [-1., 3.]);
            line([-1., 3.], [5., -4.]);
        }
        Icon::TextStyle => {
            p.text(
                c,
                Align2::CENTER_CENTER,
                "Aa",
                FontId::proportional(16.),
                stroke.color,
            );
        }
        Icon::More => {
            for x in [-6., 0., 6.] {
                p.circle_filled(c + vec2(x, 0.), 1.7, stroke.color);
            }
        }
        Icon::Sidebar => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(19., 16.)),
                2,
                stroke,
                StrokeKind::Inside,
            );
            line([-3., -7.], [-3., 7.]);
        }
        Icon::Trash => {
            p.rect_stroke(
                Rect::from_min_max(c + vec2(-6., -4.), c + vec2(6., 9.)),
                1,
                stroke,
                StrokeKind::Inside,
            );
            line([-8., -6.], [8., -6.]);
            line([-3., -9.], [3., -9.]);
            line([-2., -1.], [-2., 6.]);
            line([2., -1.], [2., 6.]);
        }
        Icon::Link => {
            p.rect_stroke(
                Rect::from_center_size(c + vec2(-4., 2.), vec2(11., 8.)),
                4,
                stroke,
                StrokeKind::Inside,
            );
            p.rect_stroke(
                Rect::from_center_size(c + vec2(4., -2.), vec2(11., 8.)),
                4,
                stroke,
                StrokeKind::Inside,
            );
            line([-3., 1.], [3., -1.]);
        }
        Icon::Attachment => {
            let points = [
                [4., -7.],
                [1., -10.],
                [-4., -8.],
                [-7., -4.],
                [-7., 5.],
                [-4., 9.],
                [1., 9.],
                [5., 5.],
                [5., -4.],
                [2., -6.],
                [-1., -4.],
                [-1., 4.],
            ];
            p.add(Shape::line(
                points.into_iter().map(|v| c + vec2(v[0], v[1])).collect(),
                stroke,
            ));
        }
    }
    response.on_hover_text(label)
}

fn save_png(path: &std::path::Path, img: &ColorImage) -> Result<(), String> {
    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(f, img.size[0] as u32, img.size[1] as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    let bytes: Vec<u8> = img.pixels.iter().flat_map(|p| p.to_array()).collect();
    w.write_image_data(&bytes).map_err(|e| e.to_string())
}
fn main() -> eframe::Result {
    let mut root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Documents/Codex/Atlas Vault");
    let mut canvas = false;
    let mut screenshot = None;
    let mut quit_after = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--vault" => {
                if let Some(v) = args.next() {
                    root = PathBuf::from(v)
                }
            }
            "--canvas" => canvas = true,
            "--write" | "--focus" | "--preview" => {}
            "--screenshot" => screenshot = args.next().map(PathBuf::from),
            "--quit-after-screenshot" => quit_after = true,
            "--help" => {
                println!(
                    "Atlas — local Markdown notes and visual canvas\n  atlas [--vault DIR] [--canvas]\n  --screenshot FILE --quit-after-screenshot   capture for verification"
                );
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {arg}");
                return Ok(());
            }
        }
    }
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(format!(
                "Atlas — {}",
                root.file_name().unwrap_or_default().to_string_lossy()
            ))
            .with_inner_size([1400., 920.])
            .with_min_inner_size([1080., 700.]),
        ..Default::default()
    };
    eframe::run_native(
        "Atlas",
        options,
        Box::new(move |cc| {
            Ok(Box::new(Atlas::new(
                cc, root, canvas, screenshot, quit_after,
            )?))
        }),
    )
}
