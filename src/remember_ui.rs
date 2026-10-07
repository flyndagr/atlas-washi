use super::*;
use crate::controls::{action, field, tab};
use atlas::remember::{Remember, valid_date};

pub(super) struct MemoryUi {
    pub capture: bool,
    pub review: bool,
    title: String,
    context: String,
    person: String,
    date: String,
    source: String,
    focus_capture: bool,
    error: Option<String>,
    filter: usize,
    today: String,
    clock_checked: Instant,
    reschedule: Option<(PathBuf, String)>,
}
impl Default for MemoryUi {
    fn default() -> Self {
        Self {
            capture: false,
            review: std::env::args().any(|a| a == "--review"),
            title: String::new(),
            context: String::new(),
            person: String::new(),
            date: String::new(),
            source: String::new(),
            focus_capture: false,
            error: None,
            filter: 0,
            today: local_date(),
            clock_checked: Instant::now(),
            reschedule: None,
        }
    }
}
fn local_date() -> String {
    std::process::Command::new("/bin/date")
        .arg("+%F")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| valid_date(s))
        .unwrap_or_default()
}
impl Atlas {
    fn begin_remember(&mut self, ctx: &Context) {
        let Some(i) = self.selected else {
            return;
        };
        let context = if self.editing {
            egui::text_edit::TextEditState::load(ctx, self.editor_id())
                .and_then(|s| s.cursor.char_range())
                .map(|r| {
                    let a = r.primary.index.min(r.secondary.index);
                    let b = r.primary.index.max(r.secondary.index);
                    self.draft
                        .chars()
                        .skip(a.into())
                        .take((b - a).into())
                        .collect::<String>()
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        self.memory.source = self.vault.notes[i].title.clone();
        self.memory.context = context;
        self.memory.title.clear();
        self.memory.person.clear();
        self.memory.date.clear();
        self.memory.error = None;
        self.memory.capture = true;
        self.memory.focus_capture = true;
        self.memory.review = false;
    }
    pub(super) fn remember_bar(&mut self, ui: &mut Ui) {
        if self.memory.clock_checked.elapsed() >= Duration::from_secs(30) {
            self.memory.today = local_date();
            self.memory.clock_checked = Instant::now();
        }
        let count = self
            .vault
            .notes
            .iter()
            .filter_map(|n| Remember::parse(&n.body))
            .filter(|r| r.due(&self.memory.today))
            .count();
        ui.horizontal_wrapped(|ui| {
            if action(ui, "How to", false)
                .on_hover_text("A guide to notes, canvas and remembering")
                .clicked()
            {
                self.guide_open = true;
                self.guide_tab = 0;
                self.memory.review = false;
            }
            if tab(ui, &format!("For today · {count}"), self.memory.review)
                .on_hover_text("Review things you chose to remember")
                .clicked()
            {
                self.memory.review = true;
                self.memory.filter = 0;
                self.memory.reschedule = None;
                self.memory.error = None;
            }
            ui.add_enabled_ui(self.selected.is_some(), |ui| {
                if action(ui, "Remember this…", false)
                    .on_hover_text("Keep a passage with its context")
                    .clicked()
                {
                    self.begin_remember(ui.ctx());
                }
            });
        });
    }

    pub(super) fn remember_windows(&mut self, ctx: &Context) {
        if self.memory.capture {
            let mut submit = false;
            let mut cancel = false;
            let modal = Modal::new(Id::new("remember_capture"))
                .frame(
                    Frame::new()
                        .fill(self.appearance.kind.sheet())
                        .stroke(Stroke::new(1., LINE))
                        .inner_margin(24)
                        .corner_radius(8),
                )
                .show(ctx, |ui| {
                    ui.set_width(460.);
                    ui.label(R::new("Remember this").font(washi::serif(30.)));
                    ui.label(
                        R::new(format!("Keep a thought from {}", self.memory.source)).color(MUTED),
                    );
                    ui.add_space(12.);
                    let title = field(
                        ui,
                        "What would you like to remember?",
                        &mut self.memory.title,
                        false,
                    );
                    if self.memory.focus_capture {
                        title.request_focus();
                        self.memory.focus_capture = false;
                    }
                    field(
                        ui,
                        "The context that matters",
                        &mut self.memory.context,
                        true,
                    );
                    field(ui, "Person · optional", &mut self.memory.person, false);
                    field(
                        ui,
                        "Review date · YYYY-MM-DD · optional",
                        &mut self.memory.date,
                        false,
                    );
                    ui.horizontal_wrapped(|ui| {
                        if action(ui, "Today", false).clicked() {
                            self.memory.date = self.memory.today.clone();
                        }
                        if action(ui, "Anytime", false).clicked() {
                            self.memory.date.clear();
                        }
                    });
                    ui.label(
                        R::new("Saved in your notebook. Review here whenever you open Atlas.")
                            .size(12.)
                            .color(MUTED),
                    );
                    if let Some(error) = &self.memory.error {
                        ui.colored_label(ACCENT, error);
                    }
                    ui.add_space(12.);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_enabled_ui(
                            !self.memory.title.trim().is_empty()
                                && !self.memory.context.trim().is_empty(),
                            |ui| {
                                submit = action(ui, "Remember", true).clicked();
                            },
                        );
                        cancel = action(ui, "Cancel", false).clicked();
                    });
                    submit |= ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter));
                });
            if cancel || modal.should_close() {
                self.memory.capture = false;
                submit = false;
            }
            if submit {
                let item = Remember {
                    title: self.memory.title.trim().into(),
                    status: "open".into(),
                    date: self.memory.date.trim().into(),
                    person: self.memory.person.trim().into(),
                    source: self.memory.source.clone(),
                    context: self.memory.context.clone(),
                };
                // Validate before saving either the source draft or the new note.
                if let Err(e) = item.markdown() {
                    self.memory.error = Some(e);
                } else if self.save() {
                    let source_path = self.selected.map(|i| self.vault.notes[i].path.clone());
                    match self.vault.remember(&item) {
                        Ok(_) => {
                            self.selected = source_path
                                .and_then(|p| self.vault.notes.iter().position(|n| n.path == p));
                            self.memory.capture = false;
                            self.memory.review = true;
                            self.memory.filter = if item.date.is_empty() {
                                1
                            } else if item.due(&self.memory.today) {
                                0
                            } else {
                                2
                            };
                            self.status = "Remembered with its original context".into();
                        }
                        Err(e) => self.memory.error = Some(e),
                    }
                } else {
                    self.memory.error = Some(
                        "Resolve the source note’s save conflict before remembering this passage."
                            .into(),
                    );
                }
            }
        }
        if !self.memory.review {
            return;
        }
        let items: Vec<_> = self
            .vault
            .notes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| Remember::parse(&n.body).map(|r| (i, n.path.clone(), r)))
            .collect();
        let mut open = true;
        let mut close = false;
        let mut change: Option<(usize, &str)> = None;
        let mut new_date: Option<(PathBuf, String)> = None;
        let mut go_source = None;
        let mut go_note = None;
        Window::new("Things to remember")
            .open(&mut open)
            .title_bar(false)
            .collapsible(false)
            .fade_in(false)
            .frame(
                Frame::new()
                    .fill(self.appearance.kind.sheet())
                    .stroke(Stroke::new(1., LINE))
                    .inner_margin(24)
                    .corner_radius(8),
            )
            .default_pos(ctx.content_rect().center() - vec2(310., 270.))
            .default_width(620.)
            .default_height(540.)
            .min_width(380.)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(R::new("Things to remember").font(washi::serif(30.)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        close = action(ui, "Close", false).clicked();
                    });
                });
                ui.label(R::new("A little attention, for what matters.").color(MUTED));
                ui.add_space(12.);
                ui.horizontal_wrapped(|ui| {
                    for (n, label) in ["For today", "Anytime", "All"].iter().enumerate() {
                        if tab(ui, label, self.memory.filter == n).clicked() {
                            self.memory.filter = n;
                            self.memory.reschedule = None;
                        }
                    }
                });
                ui.add_space(8.);
                if self.memory.today.is_empty() {
                    ui.colored_label(
                        ACCENT,
                        "Local date unavailable. Use All to review your items.",
                    );
                } else {
                    ui.label(
                        R::new(format!("{} · your notebook, your pace", self.memory.today))
                            .size(12.)
                            .color(MUTED),
                    );
                }
                if let Some(e) = &self.memory.error {
                    ui.colored_label(ACCENT, e);
                }
                let mut visible: Vec<_> = items
                    .iter()
                    .filter(|(_, _, r)| match self.memory.filter {
                        0 => r.due(&self.memory.today),
                        1 => r.status == "open" && r.date.is_empty(),
                        _ => true,
                    })
                    .collect();
                visible.sort_by(|a, b| a.2.date.cmp(&b.2.date).then(a.2.title.cmp(&b.2.title)));
                if visible.is_empty() {
                    ui.add_space(24.);
                    ui.label(
                        R::new(match self.memory.filter {
                            0 => "A little room to breathe.",
                            1 => "A place for thoughts without a deadline.",
                            _ => "Your intentions can start small.",
                        })
                        .font(washi::serif(25.)),
                    );
                    ui.label("Choose Remember this on a note to keep a thought with its context.");
                }
                ScrollArea::vertical()
                    .id_salt("remember_items")
                    .auto_shrink([false, false])
                    .max_height(430.)
                    .show(ui, |ui| {
                        for (i, path, r) in visible {
                            ui.push_id(path, |ui| {
                                ui.add_space(12.);
                                ui.separator();
                                ui.add_space(10.);
                                ui.label(R::new(&r.title).font(washi::serif(25.)));
                                let reason = if r.status == "done" {
                                    "Completed · kept for reference".into()
                                } else if r.status == "dismissed" {
                                    "Set aside · you can reopen it".into()
                                } else if r.date.is_empty() {
                                    "Anytime · no date to keep".into()
                                } else {
                                    format!("You chose {}", r.date)
                                };
                                ui.label(R::new(reason).size(12.).color(MUTED));
                                if !r.person.is_empty() {
                                    ui.label(R::new(format!("With {}", r.person)).color(GREEN));
                                }
                                ui.add_space(6.);
                                Frame::new()
                                    .fill(PANEL)
                                    .inner_margin(12)
                                    .corner_radius(5)
                                    .show(ui, |ui| {
                                        ui.set_width((ui.available_width() - 4.).max(180.));
                                        ui.label(&r.context);
                                    });
                                let source = self.vault.resolve(&r.source, Some(*i));
                                if let Some(source) = source {
                                    if ui
                                        .link(R::new(format!("From {}", r.source)).color(GREEN))
                                        .clicked()
                                    {
                                        go_source = Some(source);
                                    }
                                } else {
                                    ui.label(
                                        R::new(format!(
                                            "Source unavailable: {}. Your context is safe here.",
                                            r.source
                                        ))
                                        .size(12.)
                                        .color(MUTED),
                                    );
                                }
                                ui.add_space(6.);
                                ui.horizontal_wrapped(|ui| {
                                    if r.status == "open" {
                                        if action(ui, "Mark done", true).clicked() {
                                            change = Some((*i, "done"));
                                        }
                                    } else if action(ui, "Reopen", true).clicked() {
                                        change = Some((*i, "open"));
                                    }
                                    if action(ui, "Change date", false).clicked() {
                                        self.memory.reschedule =
                                            Some((path.clone(), r.date.clone()));
                                    }
                                    ui.menu_button("More", |ui| {
                                        if ui.button("Open remembered note").clicked() {
                                            go_note = Some(*i);
                                            ui.close();
                                        }
                                        if r.status == "open" && ui.button("Set aside").clicked() {
                                            change = Some((*i, "dismissed"));
                                            ui.close();
                                        }
                                    });
                                });
                                if let Some((target, date)) = self.memory.reschedule.as_mut()
                                    && target == path
                                {
                                    ui.add_space(8.);
                                    field(
                                        ui,
                                        "Review date · YYYY-MM-DD · blank for Anytime",
                                        date,
                                        false,
                                    );
                                    let target = target.clone();
                                    let value = date.trim().to_owned();
                                    ui.horizontal_wrapped(|ui| {
                                        if action(ui, "Save date", true).clicked() {
                                            new_date = Some((target.clone(), value));
                                        }
                                        if action(ui, "Today", false).clicked() {
                                            new_date =
                                                Some((target.clone(), self.memory.today.clone()));
                                        }
                                        if action(ui, "Anytime", false).clicked() {
                                            new_date = Some((target.clone(), String::new()));
                                        }
                                        if action(ui, "Cancel", false).clicked() {
                                            self.memory.reschedule = None;
                                        }
                                    });
                                }
                            });
                        }
                    });
            });
        self.memory.review = open && !close && !ctx.input(|i| i.key_pressed(Key::Escape));
        if !self.memory.review {
            self.memory.reschedule = None;
        }
        if let Some((path, date)) = new_date {
            if let Some(i) = self.vault.notes.iter().position(|n| n.path == path) {
                self.change_remember(i, None, Some(&date));
            } else {
                self.memory.error =
                    Some("The remembered note moved. Find it in All and try again.".into());
            }
        }
        if let Some((i, status)) = change {
            self.change_remember(i, Some(status), None);
        }
        if let Some(i) = go_source.or(go_note) {
            self.select(i);
            if self.selected == Some(i) {
                self.mode = Mode::Notes;
                self.memory.review = false;
            }
        }
    }
    fn change_remember(&mut self, i: usize, status: Option<&str>, date: Option<&str>) {
        if !self.save() {
            self.memory.error = Some("Resolve the current draft’s save conflict first.".into());
            return;
        }
        match self.vault.update_remember(i, status, date) {
            Ok(()) => {
                if self.selected == Some(i) {
                    self.draft = self.vault.notes[i].body.clone();
                }
                self.memory.reschedule = None;
                self.memory.error = None;
                self.status = "Remembered item updated".into();
            }
            Err(e) => self.memory.error = Some(e),
        }
    }
}
