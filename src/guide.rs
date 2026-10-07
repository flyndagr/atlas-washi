//! In-app help, kept close to the interactions it explains.
use super::*;
use crate::controls::{action, tab};

impl Atlas {
    pub(super) fn guide(&mut self, ctx: &Context) {
        if !self.guide_open {
            return;
        }
        let mut close = false;
        let modal = Modal::new(Id::new("atlas_guide"))
            .area(Modal::default_area(Id::new("atlas_guide")).fade_in(false))
            .backdrop_color(C::from_black_alpha(55))
            .frame(Frame::new().fill(self.appearance.kind.sheet()).stroke(Stroke::new(1., LINE)).inner_margin(24).corner_radius(8))
            .show(ctx, |ui| {
                ui.set_width(560.);
                ui.label(R::new("A little guide to Atlas").font(washi::serif(30.)));
                ui.label(R::new("Your notes, connected. Your thoughts, kept close.").color(MUTED));
                ui.add_space(12.);
                ui.horizontal_wrapped(|ui| {
                    for (index,label) in ["Notes & canvas", "Writing", "Remembering", "Your files"].iter().enumerate() {
                        if tab(ui,label,self.guide_tab == index).clicked() { self.guide_tab = index; }
                    }
                });
                ui.add_space(16.);
                ScrollArea::vertical().id_salt(("guide_body",self.guide_tab)).max_height((ctx.content_rect().height() - 250.).clamp(220., 440.))
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible).show(ui, |ui| {
                    match self.guide_tab {
                        0 => {
                            ui.label(R::new("One note. Two ways to see it.").font(washi::serif(25.)));
                            ui.label("A canvas card is a view of a Markdown note, not a separate copy. Edit the note and its card updates too.");
                            ui.add_space(12.);
                            let (rect,painter) = ui.allocate_painter(vec2(ui.available_width(),88.),Sense::hover());
                            let left = Rect::from_min_size(rect.rect.min,vec2(195.,76.));
                            let right = Rect::from_min_size(pos2(rect.rect.right()-195.,rect.rect.top()),vec2(195.,76.));
                            for (card,title,subtitle) in [(left,"Field notes","Contains [[Reading list]]"),(right,"Reading list","The linked note")] {
                                painter.rect_filled(card,5,PANEL);
                                painter.text(card.min+vec2(12.,18.),Align2::LEFT_CENTER,title,washi::serif(20.),INK);
                                painter.text(card.min+vec2(12.,49.),Align2::LEFT_CENTER,subtitle,FontId::proportional(12.),MUTED);
                            }
                            let a=left.right_center()+vec2(6.,0.); let b=right.left_center()-vec2(6.,0.);
                            painter.arrow(a,b-a,Stroke::new(1.5,GREEN));
                            ui.label(R::new("Both notes must be pinned to see the arrow.").size(12.).color(MUTED));
                            ui.add_space(12.);
                            step(ui,"1. Pin a note","Select a note in the sidebar, then click the pin icon in the top toolbar. Repeat for another note. The canvas icon switches views.");
                            step(ui,"2. Connect the ideas","Edit the first note and type [[Reading list]], using the other note’s title. Or choose Insert a link in the right panel. After saving, an arrow points from the first card to the linked card.");
                            step(ui,"3. Arrange your thoughts","Drag cards to arrange them; drag empty space to pan. Pinch or use − / + to zoom. Click the percentage to reset zoom, or the target icon to find the selected card.");
                            step(ui,"4. Return to the writing","Double-click a card to open its note. Right-click → Remove from canvas unpins it; the note and its links remain.");
                            ui.label(R::new("Arrows come from links in your notes. You don’t draw connections by dragging between cards. Card positions and zoom are saved separately.").color(MUTED));
                        }
                        1 => {
                            step(ui,"Start with a note","Choose New note, or press ⌘N. Give it a title, then write. Changes save automatically after a short pause; ⌘S saves immediately.");
                            step(ui,"Read or edit","Notes open as clean pages. The pencil opens Markdown editing; the checkmark returns to reading. Select text before using the formatting controls.");
                            step(ui,"Make a list","Choose bullets, numbers or checklists from the list controls. Enter continues a list; Enter on an empty item ends it. Change checklist completion in the Markdown source.");
                            step(ui,"Find your space","⌘K searches. The sidebar icon changes only the notes panel. Focus (⌘⇧F) hides both side panels and restores your previous layout when turned off.");
                            step(ui,"Paper & ink","The ••• menu offers paper palettes, fiber texture, fountain script and Typeset. Hover toolbar icons to see their labels.");
                        }
                        2 => {
                            step(ui,"Keep the context","While editing, select a passage and choose Remember this. In reading mode, enter the context yourself. Add the intention, an optional person and an optional review date.");
                            step(ui,"Choose when to return","Use YYYY-MM-DD, choose Today, or leave the date blank for Anytime. Saving opens the appropriate review tab.");
                            step(ui,"Review at your own pace","For today includes open items dated today or earlier. Anytime holds undated items. All also includes future, completed and set-aside items.");
                            step(ui,"Follow through","Mark done, change the date, or use More → Set aside. Reopen closed items from All. The source link returns to the original note.");
                            ui.label(R::new("Remembered items are separate Markdown notes with a captured context snapshot. This is an in-app review: Atlas must be opened to see them. There are no background alerts.").color(MUTED));
                        }
                        _ => {
                            step(ui,"Ordinary Markdown","Your notes are files in your notebook folder. Wiki links stay inside those files, so connections travel with your notes. Use the folder icon to reveal them in Finder.");
                            step(ui,"Bring work in or take it out","The document icon opens Files: import Markdown, import a folder with assets, or export a note/notebook. Single-file import copies text; folder import preserves relative assets.");
                            step(ui,"Canvas layout and backups","Notebook export keeps notes and assets but excludes hidden Atlas settings. Back up the whole notebook folder to keep card positions, zoom, appearance and Trash too.");
                            step(ui,"Recover and refresh","Deleting a note moves it to recoverable Trash in Files. Atlas checks external Markdown changes every two seconds. Unsaved drafts and conflicts pause automatic refresh.");
                            ui.label(R::new("If saving fails, keep the draft and resolve the error or save a copy. Use one Atlas window per notebook to avoid competing edits.").color(MUTED));
                        }
                    }
                });
                ui.add_space(16.);
                ui.horizontal(|ui| {
                    ui.label(R::new("Reopen this guide with How to.").size(12.).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| { close = action(ui,"Got it",true).clicked(); });
                });
            });
        if close || modal.should_close() {
            self.guide_open = false;
        }
    }
}
fn step(ui: &mut Ui, title: &str, body: &str) {
    ui.add_space(8.);
    ui.label(R::new(title).strong().color(INK));
    ui.label(body);
    ui.add_space(4.);
}
