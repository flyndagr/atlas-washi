//! Portable, explicitly captured intentions. Markdown is the source of truth.
use crate::{Result, Vault};

const MARKER: &str = "<!-- atlas:remember:v1 -->";
#[derive(Clone, Debug, PartialEq)]
pub struct Remember {
    pub title: String,
    pub status: String,
    pub date: String,
    pub person: String,
    pub source: String,
    pub context: String,
}

pub fn valid_date(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = value[..4].parse().unwrap_or(0);
    let month: usize = value[5..7].parse().unwrap_or(0);
    let day: u32 = value[8..].parse().unwrap_or(0);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && (1..=12).contains(&month) && day > 0 && day <= days[month - 1]
}
fn one_line(s: &str) -> bool {
    !s.chars().any(char::is_control)
}
impl Remember {
    pub fn markdown(&self) -> Result<String> {
        if self.title.trim().is_empty()
            || self.title.len() > 240
            || !one_line(&self.title)
            || self.person.len() > 120
            || !one_line(&self.person)
            || !one_line(&self.source)
            || self.source.is_empty()
            || self.source.contains(['[', ']', '|', '#'])
            || self.context.trim().is_empty()
            || self.context.len() > 16000
        {
            return Err("Enter an intention (up to 240 bytes) and context (up to 16 KB), with a single-line person and source.".into());
        }
        if !self.date.is_empty() && !valid_date(&self.date) {
            return Err(
                "Use a real date in YYYY-MM-DD format, or leave it blank for Anytime.".into(),
            );
        }
        if !["open", "done", "dismissed"].contains(&self.status.as_str()) {
            return Err("Unrecognized status".into());
        }
        Ok(format!(
            "{MARKER}\n# {}\nStatus: {}\nReview: {}\nPerson: {}\nSource: [[{}]]\n\n## Context\n{}\n",
            self.title.trim(),
            self.status,
            self.date,
            self.person.trim(),
            self.source,
            self.context
                .lines()
                .map(|l| format!("> {l}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
    pub fn parse(body: &str) -> Option<Self> {
        let mut lines = body.lines();
        if lines.next()? != MARKER {
            return None;
        }
        let title = lines.next()?.strip_prefix("# ")?.to_owned();
        let status = lines.next()?.strip_prefix("Status: ")?.to_owned();
        let date = lines.next()?.strip_prefix("Review: ")?.to_owned();
        let person = lines.next()?.strip_prefix("Person: ")?.to_owned();
        let source = lines
            .next()?
            .strip_prefix("Source: [[")?
            .strip_suffix("]]")?
            .to_owned();
        if !lines.next()?.is_empty() || lines.next()? != "## Context" {
            return None;
        }
        let context = lines
            .take_while(|l| l.starts_with("> "))
            .map(|l| &l[2..])
            .collect::<Vec<_>>()
            .join("\n");
        let item = Self {
            title,
            status,
            date,
            person,
            source,
            context,
        };
        item.markdown().ok()?;
        Some(item)
    }
    pub fn due(&self, today: &str) -> bool {
        self.status == "open"
            && valid_date(today)
            && !self.date.is_empty()
            && self.date.as_str() <= today
    }
}
impl Vault {
    pub fn remember(&mut self, item: &Remember) -> Result<usize> {
        let body = item.markdown()?;
        self.create_copy(&format!("Remember - {}", item.title), &body)
    }
    /// Replace only one header line; leave the context and any extra prose untouched.
    pub fn update_remember(
        &mut self,
        index: usize,
        status: Option<&str>,
        date: Option<&str>,
    ) -> Result<()> {
        let note = self
            .notes
            .get(index)
            .ok_or("Remembered item no longer exists")?;
        Remember::parse(&note.body).ok_or("This note is not a valid remembered item")?;
        if status.is_some_and(|s| !["open", "done", "dismissed"].contains(&s)) {
            return Err("Unrecognized status".into());
        }
        if date.is_some_and(|s| !s.is_empty() && !valid_date(s)) {
            return Err("Use a real date in YYYY-MM-DD format.".into());
        }
        let mut body = String::new();
        for (i, line) in note.body.split_inclusive('\n').enumerate() {
            let replacement = if i == 2 {
                status.map(|s| format!("Status: {s}"))
            } else if i == 3 {
                date.map(|d| format!("Review: {d}"))
            } else {
                None
            };
            if let Some(value) = replacement {
                body.push_str(&value);
                body.push_str(if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                });
            } else {
                body.push_str(line);
            }
        }
        self.save(index, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static ID: AtomicU64 = AtomicU64::new(0);
    fn vault() -> Vault {
        Vault::open(std::env::temp_dir().join(format!(
            "atlas-remember-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        )))
        .unwrap()
    }
    fn item() -> Remember {
        Remember {
            title: "Ask about the viewing".into(),
            status: "open".into(),
            date: "2026-10-07".into(),
            person: "Mum".into(),
            source: "Conversation".into(),
            context: "Give her time.\n日本語 café 🪴".into(),
        }
    }
    #[test]
    fn dates_and_review_buckets() {
        for d in ["2024-02-29", "2000-02-29", "2026-12-31"] {
            assert!(valid_date(d));
        }
        for d in [
            "2026-02-29",
            "1900-02-29",
            "2026-04-31",
            "2026-00-01",
            "2026-01-00",
            "0000-01-01",
            "26-1-1",
            "ééééé",
        ] {
            assert!(!valid_date(d));
        }
        let mut r = item();
        assert!(r.due("2026-10-07"));
        assert!(r.due("2026-10-08"));
        assert!(!r.due("2026-10-06"));
        r.date.clear();
        assert!(!r.due("2026-10-07"));
        r.date = "2026-10-07".into();
        r.status = "done".into();
        assert!(!r.due("2026-10-08"));
    }
    #[test]
    fn portable_context_and_lifecycle_survive_export_and_conflicts() {
        let mut v = vault();
        v.create("Conversation", "original source").unwrap();
        let i = v.remember(&item()).unwrap();
        let title = v.notes[i].title.clone();
        assert_eq!(Remember::parse(&v.notes[i].body), Some(item()));
        let extra = format!("{}\nExtra personal prose.\n", v.notes[i].body);
        v.save(i, &extra).unwrap();
        v.update_remember(i, Some("done"), None).unwrap();
        assert!(v.notes[i].body.ends_with("Extra personal prose.\n"));
        v.update_remember(i, Some("open"), Some("2026-10-10"))
            .unwrap();
        let parsed = Remember::parse(&v.notes[i].body).unwrap();
        assert!(!parsed.due("2026-10-07"));
        assert!(parsed.due("2026-10-10"));
        v.rename_note(v.find("Conversation").unwrap(), "Conversation renamed")
            .unwrap();
        let i = v.find(&title).unwrap();
        assert_eq!(
            Remember::parse(&v.notes[i].body).unwrap().source,
            "Conversation renamed"
        );
        let export = vault().root.join("export");
        v.export_notebook(&export).unwrap();
        assert!(!export.join(".atlas").exists());
        let reopened = Vault::open(export).unwrap();
        assert_eq!(
            Remember::parse(&reopened.notes[reopened.find(&title).unwrap()].body)
                .unwrap()
                .context,
            item().context
        );
        fs::write(&v.notes[i].path, "external change").unwrap();
        assert!(v.update_remember(i, Some("dismissed"), None).is_err());
        assert_eq!(
            fs::read_to_string(&v.notes[i].path).unwrap(),
            "external change"
        );
    }
    #[test]
    fn malformed_data_and_missing_sources_are_not_invented() {
        let mut v = vault();
        let i = v.remember(&item()).unwrap();
        assert!(v.resolve("Conversation", Some(i)).is_none());
        assert!(
            Remember::parse(
                &v.notes[i]
                    .body
                    .replace("Review: 2026-10-07", "Review: tomorrow")
            )
            .is_none()
        );
        let original = v.notes[i].body.clone();
        assert!(v.update_remember(i, None, Some("2026-02-30")).is_err());
        assert_eq!(v.notes[i].body, original);
        let mut r = item();
        r.person = "Mum\nStatus: done".into();
        assert!(r.markdown().is_err());
    }
}
