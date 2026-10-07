//! Plain-file storage: notes remain readable without Atlas.
pub mod editing;
pub mod files;
pub mod view;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub type Result<T> = std::result::Result<T, String>;
static SERIAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct Note {
    pub title: String,
    pub body: String,
    pub path: PathBuf,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct Board {
    #[serde(default)]
    pub positions: BTreeMap<String, [f32; 2]>,
    #[serde(default)]
    pub pan: [f32; 2],
}

impl Board {
    /// Pin in world coordinates near the current viewport; preserve existing cards.
    pub fn pin(&mut self, title: &str) -> bool {
        if self.positions.contains_key(title) {
            return false;
        }
        let mut pos = [40. - self.pan[0], 40. - self.pan[1]];
        while self
            .positions
            .values()
            .any(|p| (p[0] - pos[0]).abs() < 264. && (p[1] - pos[1]).abs() < 170.)
        {
            pos[1] += 180.;
        }
        self.positions.insert(title.into(), pos);
        true
    }
    pub fn reveal(&mut self, title: &str) {
        if let Some(p) = self.positions.get(title) {
            self.pan = [40. - p[0], 40. - p[1]];
        }
    }
}

pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub board: Board,
    board_original: Option<Vec<u8>>,
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("Missing parent folder")?;
    let temp = parent.join(format!(
        ".atlas-write-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Write a new export without replacing any existing file, including symlinks.
pub fn write_new(path: &Path, body: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            format!(
                "Cannot create {} (choose a new filename): {e}",
                path.display()
            )
        })?;
    if let Err(e) = file.write_all(body).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(path);
        return Err(e.to_string());
    }
    Ok(())
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| x.eq_ignore_ascii_case("md") || x.eq_ignore_ascii_case("markdown"))
}

pub fn wiki_links(text: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut rest = text;
    while let Some((_, tail)) = rest.split_once("[[") {
        let Some((name, after)) = tail.split_once("]]") else {
            break;
        };
        let name = name
            .split('|')
            .next()
            .unwrap_or(name)
            .split('#')
            .next()
            .unwrap_or(name)
            .trim();
        if !name.is_empty() && !links.iter().any(|x: &String| x.eq_ignore_ascii_case(name)) {
            links.push(name.to_owned());
        }
        rest = after;
    }
    links
}

pub fn tags(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for word in text.split_whitespace() {
        if let Some(tag) = word.strip_prefix('#') {
            let tag = tag.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
            if !tag.is_empty() && !tag.starts_with('#') && !found.contains(&tag.to_owned()) {
                found.push(tag.to_owned());
            }
        }
    }
    found
}

impl Vault {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(&root).map_err(|e| format!("Cannot create vault: {e}"))?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let board_path = root.join(".atlas/canvas.json");
        let original = match fs::read(&board_path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        let board: Board = match &original {
            Some(b) => serde_json::from_slice(b)
                .map_err(|e| format!("Canvas file is damaged; it has not been replaced: {e}"))?,
            None => Board::default(),
        };
        if board
            .positions
            .values()
            .flatten()
            .chain(board.pan.iter())
            .any(|x| !x.is_finite() || x.abs() > 1e7)
        {
            return Err("Canvas contains invalid coordinates".into());
        }
        let mut vault = Self {
            root,
            notes: vec![],
            board,
            board_original: original,
        };
        vault.reload()?;
        Ok(vault)
    }
    pub fn reload(&mut self) -> Result<()> {
        let mut notes = Vec::new();
        for relative in files::note_files(&self.root)? {
            let path = self.root.join(&relative);
            if !is_markdown(&path) {
                continue;
            }
            if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 5_000_000 {
                return Err(format!("{} exceeds the 5 MB note limit", path.display()));
            }
            let body = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
            let title = relative
                .with_extension("")
                .to_str()
                .ok_or("Invalid note filename")?
                .to_owned();
            notes.push(Note { title, body, path });
        }
        notes.sort_by_key(|n| n.title.to_lowercase());
        self.notes = notes;
        Ok(())
    }
    pub fn find(&self, title: &str) -> Option<usize> {
        self.notes
            .iter()
            .position(|n| n.title.eq_ignore_ascii_case(title))
    }
    pub fn search(&self, query: &str) -> Vec<usize> {
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                let hay = format!("{}\n{}", n.title, n.body).to_lowercase();
                terms.iter().all(|t| hay.contains(t))
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn create(&mut self, title: &str, body: &str) -> Result<usize> {
        let title = title.trim();
        if body.len() > 5_000_000 {
            return Err("Note exceeds the 5 MB limit".into());
        }
        if title.is_empty()
            || title.len() > 120
            || title.starts_with('.')
            || title.ends_with('.')
            || title
                .chars()
                .any(|c| c.is_control() || "/\\:[]#|".contains(c))
        {
            return Err(
                "Use a title of 1–120 bytes without / \\ : [ ] # | or leading/trailing dots".into(),
            );
        }
        if self.find(title).is_some() {
            return Err("A note with that title already exists".into());
        }
        let path = self.root.join(format!("{title}.md"));
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        f.write_all(body.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        self.notes.push(Note {
            title: title.to_owned(),
            body: body.to_owned(),
            path,
        });
        self.notes.sort_by_key(|n| n.title.to_lowercase());
        self.find(title)
            .ok_or("Note was created but could not be selected".into())
    }
    pub fn create_copy(&mut self, title: &str, body: &str) -> Result<usize> {
        let safe: String = title
            .chars()
            .map(|c| {
                if c.is_control() || "/\\:[]#|".contains(c) {
                    '-'
                } else {
                    c
                }
            })
            .collect();
        let safe = safe.trim().trim_matches('.');
        let mut base = String::new();
        for c in safe.chars() {
            if base.len() + c.len_utf8() > 90 {
                break;
            }
            base.push(c);
        }
        if base.is_empty() {
            base = "Imported note".into();
        }
        for n in 0..10000 {
            let name = if n == 0 {
                base.clone()
            } else {
                format!("{base} ({n})")
            };
            if self.find(&name).is_none() && !self.root.join(format!("{name}.md")).exists() {
                return self.create(&name, body);
            }
        }
        Err("Too many notes with the same name".into())
    }
    pub fn import_markdown(&mut self, source: &Path) -> Result<usize> {
        if !is_markdown(source) {
            return Err("Choose a .md or .markdown file".into());
        }
        if fs::metadata(source).map_err(|e| e.to_string())?.len() > 5_000_000 {
            return Err("Note exceeds the 5 MB limit".into());
        }
        let body =
            fs::read_to_string(source).map_err(|e| format!("Cannot read UTF-8 Markdown: {e}"))?;
        let name = source
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid filename")?;
        self.create_copy(name, &body)
    }
    /// Export into a newly-created directory so no destination files can be overwritten.
    pub fn export_notebook(&self, destination: &Path) -> Result<()> {
        for note in &self.notes {
            let disk = fs::read(&note.path).map_err(|e| e.to_string())?;
            if disk != note.body.as_bytes() {
                return Err(format!(
                    "{} changed outside Atlas. Reload before exporting the notebook.",
                    note.title
                ));
            }
        }
        if destination.starts_with(&self.root) {
            return Err("Export to a folder outside this notebook".into());
        }
        let files = files::tree_files(&self.root)?;
        fs::create_dir(destination).map_err(|e| format!("Choose a new export folder: {e}"))?;
        for relative in files {
            let dest = destination.join(&relative);
            fs::create_dir_all(dest.parent().ok_or("Invalid export path")?)
                .map_err(|e| e.to_string())?;
            fs::copy(self.root.join(relative), dest).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn save(&mut self, index: usize, body: &str) -> Result<()> {
        let note = self.notes.get_mut(index).ok_or("Note no longer exists")?;
        let disk = fs::read_to_string(&note.path).map_err(|e| e.to_string())?;
        if disk != note.body {
            return Err("This note changed outside Atlas. Your draft is still here. Save a copy before reloading.".into());
        }
        if body.len() > 5_000_000 {
            return Err("Note exceeds the 5 MB limit".into());
        }
        atomic_write(&note.path, body.as_bytes())?;
        note.body = body.to_owned();
        Ok(())
    }
    pub fn save_board(&mut self) -> Result<()> {
        let dir = self.root.join(".atlas");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("canvas.json");
        let disk = match fs::read(&path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if disk != self.board_original {
            return Err("Canvas changed outside Atlas; restart to reload it. Existing canvas was preserved.".into());
        }
        let bytes = serde_json::to_vec_pretty(&self.board).map_err(|e| e.to_string())?;
        atomic_write(&path, &bytes)?;
        self.board_original = Some(bytes);
        Ok(())
    }
    pub fn backlinks(&self, title: &str) -> Vec<usize> {
        self.notes
            .iter()
            .enumerate()
            .filter(|(i, n)| {
                wiki_links(&n.body)
                    .iter()
                    .any(|l| self.resolve(l, Some(*i)) == self.find(title))
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn attach(&self, source: &Path) -> Result<String> {
        if !source.is_file() {
            return Err("Choose a file to attach".into());
        }
        let name = source
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("Invalid attachment filename")?;
        let safe: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || ".-_".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let dir = self.root.join("attachments");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dir = dir.canonicalize().map_err(|e| e.to_string())?;
        if !dir.starts_with(&self.root) {
            return Err("Attachments directory must be inside the vault".into());
        }
        for i in 0..10000 {
            let filename = if i == 0 {
                safe.clone()
            } else {
                format!("{i}-{safe}")
            };
            let target = dir.join(&filename);
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
            {
                Ok(mut f) => {
                    let mut input = fs::File::open(source).map_err(|e| e.to_string())?;
                    std::io::copy(&mut input, &mut f)
                        .and_then(|_| f.sync_all())
                        .map_err(|e| e.to_string())?;
                    return Ok(format!("[{filename}](attachments/{filename})"));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string()),
            }
        }
        Err("Too many attachments with the same name".into())
    }
    pub fn seed(&mut self) -> Result<()> {
        if !self.notes.is_empty() || self.root.join(".atlas/trash").exists() {
            return Ok(());
        }
        let notes = [
            (
                "Start here",
                "# A little space for big ideas.\n\nWelcome to **Atlas**. A quiet home for your notes, sources, and connections. Everything here is an ordinary Markdown file on your Mac.\n\n## Follow your curiosity\n\nStart with a question. Collect what catches your eye. Connect the ideas that belong together.\n\n- Write in the editor, then switch to **Read**.\n- Link two notes with [[double brackets]].\n- Pin a note to the **Canvas** and move it anywhere.\n- Find words or #tags with the search field.\n\n## Your first expedition\n\nExplore [[The curiosity loop]], collect a few [[Field notes]], or open [[Reading list]].\n\n> The best notes are the ones that lead somewhere new.\n\n#guide #welcome\n",
            ),
            (
                "The curiosity loop",
                "# The curiosity loop\n\nGood research is a conversation between questions and evidence.\n\n## A simple practice\n\n1. Notice something you cannot quite explain.\n2. Write a question in your own words.\n3. Gather an observation, a source, and a counterexample.\n4. Connect it to an idea you already have.\n5. Return later and see what changed.\n\nKeep the raw material in [[Field notes]]. Let [[Questions worth keeping]] guide the next step.\n\n#research #ideas\n",
            ),
            (
                "Field notes",
                "# Field notes\n\nA place for small observations before they become big ideas.\n\n## Today I noticed\n\n- A good tool leaves room for the person using it.\n- Putting two unrelated ideas next to each other can change both.\n- A question is often more useful than a summary.\n\n## Next experiment\n\nSpend ten minutes collecting details without judging them. Then revisit [[The curiosity loop]].\n\n#observations #research\n",
            ),
            (
                "Reading list",
                "# Reading list\n\nRead slowly. Keep what changes your mind.\n\n## On the desk\n\n- [ ] A source that challenges a current belief\n- [ ] A deep dive into a new subject\n- [x] A note about how to take better notes\n\n## For each source\n\nRecord the title, a link, and one idea in your own words. Use **Attach file** to keep a local PDF or image with your notes.\n\nSend new questions to [[Questions worth keeping]].\n\n#sources #reading\n",
            ),
            (
                "Questions worth keeping",
                "# Questions worth keeping\n\nNot every question needs an immediate answer.\n\n- What makes a creative tool feel like an extension of your thinking?\n- Which ideas keep returning in my [[Field notes]]?\n- What would I explore if I had an uninterrupted afternoon?\n\n## Working hypothesis\n\nThe connections are often more valuable than the collection. Try arranging these notes on the canvas.\n\n#questions #ideas\n",
            ),
        ];
        for (i, (title, body)) in notes.iter().enumerate() {
            self.create(title, body)?;
            self.board.positions.insert(
                title.to_string(),
                [55.0 + (i % 2) as f32 * 330.0, 60.0 + (i / 2) as f32 * 210.0],
            );
        }
        self.save_board()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vault() -> Vault {
        Vault::open(std::env::temp_dir().join(format!(
            "atlas-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        )))
        .unwrap()
    }
    #[test]
    fn folder_import_preserves_assets_scopes_links_and_exports_structure() {
        let mut v = vault();
        let source = vault().root;
        fs::create_dir_all(source.join("Research/images")).unwrap();
        fs::write(
            source.join("Research/Plan.md"),
            "# Plan\n[[Other]] ![art](images/art.png) [other](Other.md)",
        )
        .unwrap();
        fs::write(source.join("Research/Other.md"), "related").unwrap();
        fs::write(source.join("Research/images/art.png"), b"asset bytes").unwrap();
        v.create("Other", "outside import").unwrap();
        let (prefix, count) = v.import_folder(&source).unwrap();
        assert_eq!(count, 2);
        let plan = v.find(&format!("{prefix}/Research/Plan")).unwrap();
        let other = v.find(&format!("{prefix}/Research/Other")).unwrap();
        assert_eq!(v.resolve("Other", Some(plan)), Some(other));
        assert_eq!(v.resolve("Other.md", Some(plan)), Some(other));
        assert_eq!(
            fs::read(v.root.join(&prefix).join("Research/images/art.png")).unwrap(),
            b"asset bytes"
        );
        let (second, _) = v.import_folder(&source).unwrap();
        assert_ne!(prefix, second);
        let export = vault().root.join("export");
        v.export_notebook(&export).unwrap();
        let reopened = Vault::open(export).unwrap();
        assert!(reopened.find(&format!("{prefix}/Research/Plan")).is_some());
        assert_eq!(
            fs::read_to_string(source.join("Research/Other.md")).unwrap(),
            "related"
        );
    }
    #[test]
    fn nested_copies_and_recovered_drafts_keep_relative_assets() {
        let mut v = vault();
        fs::create_dir_all(v.root.join("Project/images")).unwrap();
        fs::write(v.root.join("Project/Note.md"), "[art](images/a.txt)").unwrap();
        fs::write(v.root.join("Project/images/a.txt"), "asset").unwrap();
        v.reload().unwrap();
        let i = v.find("Project/Note").unwrap();
        let copy = v.create_neighbor(i, "[art](images/a.txt)", "copy").unwrap();
        assert_eq!(v.notes[copy].title, "Project/Note copy");
        let i = v.find("Project/Note").unwrap();
        let recovered = v.create_neighbor(i, "draft", "recovered").unwrap();
        assert_eq!(v.notes[recovered].title, "Project/Note recovered");
    }
    #[test]
    fn rename_rewrites_resolved_links_preserves_examples_and_canvas() {
        let mut v = vault();
        v.create("Old", "# Old\n").unwrap();
        v.create(
            "Ref",
            "[[Old|alias]] [[Old#section]] [old](Old.md#section) `[[Old]]`\n\n```\n[[Old]]\n```\n",
        )
        .unwrap();
        v.board.positions.insert("Old".into(), [5., 6.]);
        v.save_board().unwrap();
        let i = v.rename_note(v.find("Old").unwrap(), "New name").unwrap();
        assert_eq!(v.notes[i].title, "New name");
        assert!(!v.root.join("Old.md").exists());
        let body = &v.notes[v.find("Ref").unwrap()].body;
        assert!(body.contains("[[New name|alias]]"));
        assert!(body.contains("[[New name#section]]"));
        assert!(body.contains("(New%20name.md#section)"));
        assert!(body.contains("`[[Old]]`"));
        assert!(body.contains("```\n[[Old]]\n```"));
        assert_eq!(v.board.positions["New name"], [5., 6.]);
    }
    #[test]
    fn trash_restore_collision_and_restart_preserve_both_notes() {
        let mut v = vault();
        let i = v.create("Personal", "keep me").unwrap();
        v.trash_note(i).unwrap();
        assert!(v.notes.is_empty());
        let mut v = Vault::open(v.root.clone()).unwrap();
        let record = v.trashed_notes().unwrap().remove(0);
        v.create("Personal", "new version").unwrap();
        assert!(v.restore_note(&record.id).is_err());
        assert_eq!(
            fs::read_to_string(v.root.join("Personal.md")).unwrap(),
            "new version"
        );
        v.rename_note(v.find("Personal").unwrap(), "New version")
            .unwrap();
        let i = v.restore_note(&record.id).unwrap();
        assert_eq!(v.notes[i].body, "keep me");
        assert!(v.trashed_notes().unwrap().is_empty());
    }
    #[test]
    fn markdown_round_trip_preserves_unicode_syntax_and_attachments() {
        let mut v = vault();
        let source_dir = vault().root;
        let source = source_dir.join("Journal.MD");
        let body = "---\r\ntags: [zen]\r\n---\r\n# 日本語 café\r\n\r\n- [ ] task\r\n| A | B |\r\n|---|---|\r\n| 1 | 2 |\r\n[[Other]] ![art](attachments/art.txt)\r\n```rust\r\nfn main() {}\r\n```\r\n";
        fs::write(&source, body).unwrap();
        let i = v.import_markdown(&source).unwrap();
        assert_eq!(v.notes[i].body, body);
        assert_eq!(fs::read_to_string(&source).unwrap(), body);
        let edited = format!("{body}\nEdited and saved.\n");
        v.save(i, &edited).unwrap();
        let attachment = source_dir.join("art.txt");
        fs::write(&attachment, "asset content").unwrap();
        v.attach(&attachment).unwrap();
        let export = vault().root.join("export");
        v.export_notebook(&export).unwrap();
        assert_eq!(
            fs::read_to_string(export.join("Journal.md")).unwrap(),
            edited
        );
        assert_eq!(
            fs::read_to_string(export.join("attachments/art.txt")).unwrap(),
            "asset content"
        );
        let mut reopened = Vault::open(export).unwrap();
        assert_eq!(reopened.notes[0].body, edited);
        let copy = reopened.create_copy("Journal", &edited).unwrap();
        assert_eq!(reopened.notes[copy].title, "Journal (1)");
    }
    #[test]
    fn import_collisions_bad_inputs_and_export_refusal_preserve_data() {
        let mut v = vault();
        let incoming = vault().root;
        let source = incoming.join("A.markdown");
        fs::write(&source, "imported").unwrap();
        v.create("A", "original").unwrap();
        let i = v.import_markdown(&source).unwrap();
        assert_eq!(v.notes[i].title, "A (1)");
        assert_eq!(v.notes[v.find("A").unwrap()].body, "original");
        let bad = incoming.join("bad.md");
        fs::write(&bad, [255, 254]).unwrap();
        assert!(v.import_markdown(&bad).is_err());
        let large = incoming.join("large.md");
        fs::write(&large, vec![b'x'; 5_000_001]).unwrap();
        assert!(v.import_markdown(&large).is_err());
        assert!(v.create("Large", &"x".repeat(5_000_001)).is_err());
        assert!(write_new(&source, b"replacement").is_err());
        assert_eq!(fs::read_to_string(source).unwrap(), "imported");
        assert!(v.export_notebook(&incoming).is_err());
        assert_eq!(v.notes.len(), 2);
    }
    #[test]
    fn notebook_export_refuses_stale_notes_before_creating_destination() {
        let mut v = vault();
        let i = v.create("A", "original").unwrap();
        fs::write(&v.notes[i].path, "external").unwrap();
        let dest = vault().root.join("export");
        assert!(v.export_notebook(&dest).is_err());
        assert!(!dest.exists());
        v.reload().unwrap();
        v.export_notebook(&dest).unwrap();
        assert_eq!(fs::read_to_string(dest.join("A.md")).unwrap(), "external");
    }
    #[test]
    fn markdown_extensions_and_export_symlinks() {
        let mut v = vault();
        fs::write(v.root.join("Upper.MD"), "upper").unwrap();
        fs::write(v.root.join("Long.markdown"), "long").unwrap();
        v.reload().unwrap();
        assert_eq!(v.notes.len(), 2);
        #[cfg(unix)]
        {
            let outside = v.root.join("outside.txt");
            fs::write(&outside, "private").unwrap();
            fs::create_dir(v.root.join("attachments")).unwrap();
            std::os::unix::fs::symlink(&outside, v.root.join("attachments/link")).unwrap();
            let export = vault().root.join("export");
            assert!(v.export_notebook(&export).is_err());
            assert!(!export.join("attachments/link").exists());
            assert_eq!(fs::read_to_string(outside).unwrap(), "private");
        }
    }
    #[test]
    fn notes_persist_and_conflicts_preserve_both_versions() {
        let mut v = vault();
        let i = v.create("First", "original").unwrap();
        v.save(i, "edited").unwrap();
        assert_eq!(Vault::open(v.root.clone()).unwrap().notes[0].body, "edited");
        fs::write(&v.notes[i].path, "external").unwrap();
        assert!(v.save(i, "draft").is_err());
        assert_eq!(fs::read_to_string(&v.notes[i].path).unwrap(), "external");
        assert_eq!(v.notes[i].body, "edited");
    }
    #[test]
    fn search_links_and_backlinks() {
        let mut v = vault();
        v.create("Alpha", "A #research note about forests").unwrap();
        v.create("Beta", "See [[Alpha|first]] and [[Alpha#section]]")
            .unwrap();
        assert_eq!(v.search("RESEARCH forest").len(), 1);
        assert_eq!(v.backlinks("alpha").len(), 1);
        assert_eq!(wiki_links(&v.notes[1].body), vec!["Alpha"]);
    }
    #[test]
    fn rejects_unsafe_and_duplicate_names() {
        let mut v = vault();
        for name in ["", "../escape", "a/b", ".hidden", "a\\b"] {
            assert!(v.create(name, "").is_err());
        }
        v.create("Safe", "").unwrap();
        assert!(v.create("safe", "").is_err());
    }
    #[test]
    fn canvas_roundtrip_and_conflict() {
        let mut v = vault();
        v.board.positions.insert("Idea".into(), [12., 34.]);
        v.save_board().unwrap();
        let mut second = Vault::open(v.root.clone()).unwrap();
        assert_eq!(second.board.positions["Idea"], [12., 34.]);
        v.board.pan = [10., 20.];
        v.save_board().unwrap();
        assert!(second.save_board().is_err());
    }
    #[test]
    fn seed_does_not_overwrite_existing_notes() {
        let mut v = vault();
        v.create("Personal", "keep me").unwrap();
        v.seed().unwrap();
        assert_eq!(v.notes.len(), 1);
        assert_eq!(v.notes[0].body, "keep me");
    }
    #[test]
    fn attachments_never_overwrite() {
        let v = vault();
        let input = v.root.join("source.txt");
        fs::write(&input, "first").unwrap();
        assert!(v.attach(&input).unwrap().contains("attachments/source.txt"));
        fs::write(&input, "second").unwrap();
        assert!(
            v.attach(&input)
                .unwrap()
                .contains("attachments/1-source.txt")
        );
        assert_eq!(
            fs::read_to_string(v.root.join("attachments/source.txt")).unwrap(),
            "first"
        );
    }
}

#[cfg(test)]
mod board_interaction_tests {
    use super::*;
    #[test]
    fn pin_is_unique_nonoverlapping_and_reveal_handles_panning() {
        let mut b = Board {
            pan: [500., -300.],
            ..Default::default()
        };
        assert!(b.pin("First"));
        assert_eq!(b.positions["First"], [-460., 340.]);
        assert!(!b.pin("First"));
        assert!(b.pin("Second"));
        assert!((b.positions["Second"][1] - b.positions["First"][1]).abs() >= 170.);
        b.pan = [999., 999.];
        b.reveal("First");
        assert_eq!(b.pan, [500., -300.]);
    }
}
