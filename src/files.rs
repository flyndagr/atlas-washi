//! Folder-preserving imports and recoverable note management.
use super::*;
use std::path::Component;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrashedNote {
    pub id: String,
    pub title: String,
    pub relative_path: PathBuf,
}

pub fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    let mut out = PathBuf::new();
    for _ in common..a.len() {
        out.push("..");
    }
    for component in &b[common..] {
        out.push(component.as_os_str());
    }
    out
}
fn normalize(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::Normal(s) => out.push(s),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    Some(out)
}
pub fn decode_path(s: &str) -> String {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(a), Some(b)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            )
        {
            out.push((a * 16 + b) as u8);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.into())
}
pub fn encode_path(path: &Path) -> String {
    let mut out = String::new();
    for b in path.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
/// Files chosen for imports/exports. Hidden metadata and symlinks are never followed.
pub fn tree_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn walk(
        root: &Path,
        dir: &Path,
        depth: usize,
        out: &mut Vec<PathBuf>,
        bytes: &mut u64,
    ) -> Result<()> {
        if depth > 32 {
            return Err("Folder nesting exceeds 32 levels".into());
        }
        for item in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            if item.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let kind = item.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err(format!(
                    "Symbolic link is not supported: {}",
                    item.path().display()
                ));
            }
            if kind.is_dir() {
                walk(root, &item.path(), depth + 1, out, bytes)?;
            } else if kind.is_file() {
                *bytes += item.metadata().map_err(|e| e.to_string())?.len();
                if *bytes > 500_000_000 || out.len() >= 10000 {
                    return Err("Choose a folder under 500 MB and 10,000 files".into());
                }
                out.push(
                    item.path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_owned(),
                );
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(root, root, 0, &mut files, &mut 0)?;
    files.sort();
    Ok(files)
}
pub fn note_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) -> Result<()> {
        if depth > 32 {
            return Err("Folder nesting exceeds 32 levels".into());
        }
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_dir() {
                walk(root, &entry.path(), depth + 1, out)?;
            } else if kind.is_file() && is_markdown(&entry.path()) {
                out.push(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .into(),
                );
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(root, root, 0, &mut files)?;
    files.sort();
    Ok(files)
}
impl Vault {
    pub fn resolve(&self, target: &str, from: Option<usize>) -> Option<usize> {
        let raw = target.split('|').next()?.split('#').next()?.trim();
        let raw = decode_path(raw);
        if raw.contains("://") {
            return None;
        }
        let mut path = PathBuf::from(&raw);
        if is_markdown(&path) {
            path.set_extension("");
        }
        let find = |path: &Path| self.find(&path.to_string_lossy());
        let package = from.and_then(|i| self.notes.get(i)).and_then(|n| {
            let mut parts = Path::new(&n.title).components();
            if parts.next()?.as_os_str() != "imports" {
                return None;
            }
            Some(PathBuf::from("imports").join(parts.next()?.as_os_str()))
        });
        if let Some(i) = from
            && let Some(n) = self.notes.get(i)
        {
            let parent = Path::new(&n.title).parent().unwrap_or(Path::new(""));
            if let Some(local) = normalize(&parent.join(&path))
                && let Some(i) = find(&local)
            {
                return Some(i);
            }
        }
        if let Some(package) = &package
            && let Some(scoped) = normalize(&package.join(&path))
            && let Some(i) = find(&scoped)
        {
            return Some(i);
        }
        if let Some(direct) = normalize(&path)
            && let Some(i) = find(&direct)
        {
            return Some(i);
        }
        let matches: Vec<_> = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| {
                Path::new(&n.title).file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .eq_ignore_ascii_case(&path.to_string_lossy())
                })
            })
            .collect();
        if let Some(package) = package {
            let scoped: Vec<_> = matches
                .iter()
                .filter(|(_, n)| Path::new(&n.title).starts_with(&package))
                .collect();
            if scoped.len() == 1 {
                return Some(scoped[0].0);
            }
        }
        if matches.len() == 1 {
            Some(matches[0].0)
        } else {
            None
        }
    }
    pub fn import_folder(&mut self, source: &Path) -> Result<(String, usize)> {
        let source = source.canonicalize().map_err(|e| e.to_string())?;
        if source.starts_with(&self.root) || self.root.starts_with(&source) {
            return Err("Choose a source folder outside this notebook".into());
        }
        let files = tree_files(&source)?;
        let mut count = 0;
        for file in &files {
            if is_markdown(file) {
                let path = source.join(file);
                if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 5_000_000 {
                    return Err(format!("{} exceeds 5 MB", file.display()));
                }
                fs::read_to_string(path)
                    .map_err(|e| format!("Invalid UTF-8 in {}: {e}", file.display()))?;
                count += 1;
            }
        }
        if count == 0 {
            return Err("No Markdown files found in that folder".into());
        }
        let base: String = source
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || "-_ ".contains(c) {
                    c
                } else {
                    '-'
                }
            })
            .take(60)
            .collect();
        let base = if base.trim().is_empty() {
            "Imported notebook"
        } else {
            base.trim()
        };
        let parent = self.root.join("imports");
        fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        if !parent
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(&self.root)
        {
            return Err("Imports folder must stay inside the notebook".into());
        }
        let mut name = base.to_owned();
        let mut n = 2;
        while parent.join(&name).exists() {
            name = format!("{base} ({n})");
            n += 1;
        }
        let staging = self.root.join(format!(
            ".atlas-import-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staging).map_err(|e| e.to_string())?;
        let result = (|| {
            for file in &files {
                let dest = staging.join(file);
                fs::create_dir_all(dest.parent().ok_or("Invalid path")?)
                    .map_err(|e| e.to_string())?;
                fs::copy(source.join(file), dest).map_err(|e| e.to_string())?;
            }
            fs::rename(&staging, parent.join(&name)).map_err(|e| e.to_string())?;
            self.reload()?;
            Ok((format!("imports/{name}"), count))
        })();
        if staging.exists() {
            let _ = fs::remove_dir_all(staging);
        } // only this operation's unpublished staging tree
        result
    }
    pub fn create_neighbor(&mut self, index: usize, body: &str, suffix: &str) -> Result<usize> {
        let note = self.notes.get(index).ok_or("Missing note")?;
        if body.len() > 5_000_000 {
            return Err("Note exceeds 5 MB".into());
        }
        let stem = note
            .path
            .file_stem()
            .ok_or("Invalid filename")?
            .to_string_lossy();
        let parent = note.path.parent().ok_or("Invalid parent")?;
        for n in 1..10000 {
            let name = if n == 1 {
                format!("{stem} {suffix}")
            } else {
                format!("{stem} {suffix} ({n})")
            };
            let path = parent.join(format!("{name}.md"));
            if path.exists() {
                continue;
            }
            write_new(&path, body.as_bytes())?;
            let title = path
                .strip_prefix(&self.root)
                .map_err(|e| e.to_string())?
                .with_extension("")
                .to_string_lossy()
                .into_owned();
            self.reload()?;
            return self.find(&title).ok_or("New note missing".into());
        }
        Err("Too many note copies".into())
    }
    pub fn trash_note(&mut self, index: usize) -> Result<()> {
        let note = self.notes.get(index).ok_or("Note not found")?;
        if fs::read(&note.path).map_err(|e| e.to_string())? != note.body.as_bytes() {
            return Err("Note changed outside Atlas. Reload before moving it to Trash.".into());
        }
        let id = format!(
            "{}-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_millis(),
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let dir = self.root.join(".atlas/trash").join(&id);
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let record = TrashedNote {
            id,
            title: note.title.clone(),
            relative_path: note
                .path
                .strip_prefix(&self.root)
                .map_err(|e| e.to_string())?
                .into(),
        };
        write_new(
            &dir.join("record.json"),
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
        )?;
        fs::rename(&note.path, dir.join("note.md")).map_err(|e| e.to_string())?;
        self.notes.remove(index);
        Ok(())
    }
    pub fn trashed_notes(&self) -> Result<Vec<TrashedNote>> {
        let root = self.root.join(".atlas/trash");
        if !root.exists() {
            return Ok(vec![]);
        }
        let mut notes = Vec::new();
        for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().map_err(|e| e.to_string())?.is_dir()
                && entry.path().join("note.md").is_file()
            {
                let mut record: TrashedNote = serde_json::from_slice(
                    &fs::read(entry.path().join("record.json")).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                record.id = entry.file_name().to_string_lossy().into_owned();
                notes.push(record);
            }
        }
        notes.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(notes)
    }
    pub fn restore_note(&mut self, id: &str) -> Result<usize> {
        let record = self
            .trashed_notes()?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or("Trash entry missing")?;
        let rel = normalize(&record.relative_path).ok_or("Invalid trash path")?;
        let destination = self.root.join(rel);
        let parent = destination.parent().ok_or("Invalid destination")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        if !parent
            .canonicalize()
            .map_err(|e| e.to_string())?
            .starts_with(&self.root)
        {
            return Err("Restore destination is outside notebook".into());
        }
        let source = self.root.join(".atlas/trash").join(id).join("note.md");
        fs::hard_link(&source, &destination)
            .map_err(|e| format!("Cannot restore (an existing note is never replaced): {e}"))?;
        fs::remove_file(source).map_err(|e| e.to_string())?;
        self.reload()?;
        self.find(&record.title)
            .ok_or("Restored note could not be selected".into())
    }
    pub fn rename_note(&mut self, index: usize, name: &str) -> Result<usize> {
        let name = name.trim();
        if name.is_empty()
            || name.len() > 120
            || name.starts_with('.')
            || name.ends_with('.')
            || name
                .chars()
                .any(|c| c.is_control() || "/\\:[]#|".contains(c))
        {
            return Err("Choose a filename without slashes, brackets, #, or leading dots".into());
        }
        let old = self.notes.get(index).ok_or("Missing note")?.clone();
        let destination = old.path.with_file_name(format!(
            "{name}.{}",
            old.path.extension().unwrap_or_default().to_string_lossy()
        ));
        if old.path == destination {
            return Ok(index);
        }
        if destination.exists() {
            return Err("A file with that name already exists".into());
        }
        let new_title = destination
            .strip_prefix(&self.root)
            .map_err(|e| e.to_string())?
            .with_extension("")
            .to_string_lossy()
            .into_owned();
        // Check all files first; backups make every original recoverable if an OS write fails.
        for note in &self.notes {
            if fs::read(&note.path).map_err(|e| e.to_string())? != note.body.as_bytes() {
                return Err("A note changed outside Atlas. Reload before renaming.".into());
            }
        }
        let mut changes = Vec::new();
        for (i, note) in self.notes.iter().enumerate() {
            let body = rewrite_note_links(self, i, index, &new_title, &destination);
            if body != note.body {
                changes.push((i, body));
            }
        }
        let backup = self.root.join(".atlas/rename-backups").join(format!(
            "{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&backup).map_err(|e| e.to_string())?;
        for (i, _) in &changes {
            let note = &self.notes[*i];
            let dest = backup.join(
                note.path
                    .strip_prefix(&self.root)
                    .map_err(|e| e.to_string())?,
            );
            fs::create_dir_all(dest.parent().ok_or("Bad backup path")?)
                .map_err(|e| e.to_string())?;
            write_new(&dest, note.body.as_bytes())?;
        }
        let renamed_body = changes
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, b)| b.as_str())
            .unwrap_or(&old.body);
        write_new(&destination, renamed_body.as_bytes())?;
        let board_before = self.board.clone();
        let result: Result<()> = (|| {
            for (i, body) in &changes {
                if *i != index {
                    self.save(*i, body)?;
                }
            }
            if let Some(position) = self.board.positions.remove(&old.title) {
                self.board.positions.insert(new_title.clone(), position);
                self.save_board()?;
            }
            fs::remove_file(&old.path).map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(e) = result {
            for (i, _) in &changes {
                if *i != index {
                    let note = &self.notes[*i];
                    let original = backup.join(
                        note.path
                            .strip_prefix(&self.root)
                            .map_err(|e| e.to_string())?,
                    );
                    if let Ok(bytes) = fs::read(original) {
                        let _ = atomic_write(&note.path, &bytes);
                    }
                }
            }
            self.board = board_before;
            let _ = self.save_board();
            let _ = fs::remove_file(destination);
            let _ = self.reload();
            return Err(format!(
                "Rename failed: {e}. Originals are also backed up at {}",
                backup.display()
            ));
        }
        self.reload()?;
        self.find(&new_title).ok_or("Renamed note missing".into())
    }
}
fn rewrite_note_links(
    vault: &Vault,
    from: usize,
    target: usize,
    title: &str,
    destination: &Path,
) -> String {
    use pulldown_cmark::{Event, Parser, Tag};
    let text = &vault.notes[from].body;
    let mut excluded = Vec::new();
    let mut replacements = Vec::new();
    for (event, range) in Parser::new(text).into_offset_iter() {
        match event {
            Event::Code(_)
            | Event::Start(Tag::CodeBlock(_))
            | Event::Html(_)
            | Event::InlineHtml(_) => excluded.push(range),
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. })
                if vault.resolve(&dest_url, Some(from)) == Some(target) =>
            {
                let markup = &text[range.clone()];
                // Inline destinations only; reference-style definitions remain untouched.
                if let Some(start) = markup.rfind("](") {
                    let suffix = dest_url.find('#').map(|i| &dest_url[i..]).unwrap_or("");
                    let path = relative_path(
                        vault.notes[from].path.parent().unwrap_or(&vault.root),
                        destination,
                    );
                    if let Some(offset) = markup[start + 2..].find(dest_url.as_ref()) {
                        let start = range.start + start + 2 + offset;
                        replacements.push((
                            start..start + dest_url.len(),
                            format!("{}{suffix}", encode_path(&path)),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    let mut cursor = 0;
    while let Some(start) = text[cursor..].find("[[") {
        let start = cursor + start;
        let Some(end) = text[start + 2..].find("]]") else {
            break;
        };
        let end = start + 2 + end;
        if !excluded.iter().any(|r| r.contains(&start)) && !text[..start].ends_with('\\') {
            let content = &text[start + 2..end];
            let name = content.split(['#', '|']).next().unwrap_or(content);
            if vault.resolve(name, Some(from)) == Some(target) {
                replacements.push((start + 2..start + 2 + name.len(), title.into()));
            }
        }
        cursor = end + 2;
    }
    replacements.sort_by_key(|(range, _)| range.start);
    let mut result = text.clone();
    for (range, value) in replacements.into_iter().rev() {
        result.replace_range(range, &value);
    }
    result
}
