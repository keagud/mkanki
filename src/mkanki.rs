use genanki_rs::{
    basic_model, basic_type_in_the_answer_model, cloze_model, Deck, Field, Model, Note,
};

use expanduser::expanduser;
use glob::glob;
use itertools::Itertools;
use markdown::to_html;
use regex::Captures;
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;

lazy_static::lazy_static! {

    static ref HEADER_PATTERN: regex::Regex = regex::Regex::new(r#"^##\s+(.+)"#).unwrap();
    static ref COMMENT_PATTERN: regex::Regex = regex::Regex::new(r#"^\s*<!--.*?-->\s*$"#).unwrap();
    static ref CLOZE_PATTERN: regex::Regex = regex::Regex::new(r#"\{\{(.+?)}}"#).unwrap();


    static ref TYPE_IN_ANSWER_MODEL: genanki_rs::Model = genanki_rs::basic_type_in_the_answer_model();

    static ref CLOZE_MODEL: genanki_rs::Model = genanki_rs::cloze_model();
    static ref BASIC_MODEL: genanki_rs::Model = genanki_rs::basic_model();
}

type DecksCollection = std::collections::HashMap<String, Deck>;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeckConfig {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,

    #[serde(default)]
    pub is_default: bool,

    #[serde(default)]
    type_in_prefixes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ConfigAll {
    type_in_prefixes: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    all: ConfigAll,
    decks: Vec<DeckConfig>,
}

impl DeckConfig {
    pub fn as_deck(&self) -> Deck {
        let desc = if let Some(d) = &self.description {
            d.clone()
        } else {
            String::new()
        };

        genanki_rs::Deck::new(self.id, &self.name, &desc)
    }
}

impl From<DeckConfig> for genanki_rs::Deck {
    fn from(val: DeckConfig) -> Self {
        genanki_rs::Deck::new(val.id, &val.name, &val.description.unwrap_or_default())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct NoteFields {
    header: String,
    body_lines: Vec<String>,
}

impl NoteFields {
    pub fn to_note(&self, deck_config: &DeckConfig) -> crate::Result<Note> {
        let body_text = self.body_lines.join("\n");

        let header_html = to_html(&format!("## {}", self.header));

        let note: Note = if deck_config
            .type_in_prefixes
            .iter()
            .any(|p| self.header.starts_with(p))
        {
            Note::new(
                TYPE_IN_ANSWER_MODEL.to_owned(),
                vec![&header_html, &body_text],
            )?
        } else {
            let html_body_text = to_html(body_text.as_str());
            let full_html_text = format!("{header_html}\n{html_body_text}");

            if let Some(clozes) = process_clozes(full_html_text.as_ref()) {
                Note::new(CLOZE_MODEL.to_owned(), vec![clozes.as_ref()])?
            } else {
                Note::new(BASIC_MODEL.to_owned(), vec![&header_html, &html_body_text])?
            }
        };


        Ok(note)
    }
}

/// convert {{unnumbered}} {{clozes}} to {{c1::numbered}} {{c2::clozes}}
pub fn process_clozes(cloze_text: &str) -> Option<Cow<'_, str>> {
    let mut counter = 0usize;

    let rep = CLOZE_PATTERN.replace_all(cloze_text.as_ref(), |c: &Captures| -> String {
        counter += 1;
        let text = c
            .get(1)
            .map(|m| m.as_str())
            .expect("Cloze pattern could not be processed");
        ["{{c", &counter.to_string(), "::", text, "}}"]
            .into_iter()
            .join("")
    });

    if counter > 0 {
        Some(rep)
    } else {
        None
    }
}

pub fn read_md_file(path: impl AsRef<Path>) -> crate::Result<Vec<NoteFields>> {
    let mut notes = Vec::new();
    let mut current_note: Option<NoteFields> = None;

    for line in std::fs::read_to_string(path.as_ref())?.lines() {
        let line_trimmed = line.trim();

        if line_trimmed.is_empty() || COMMENT_PATTERN.is_match(line_trimmed) {
            continue;
        }

        if let Some(header_text) = HEADER_PATTERN.captures(line_trimmed).and_then(|c| c.get(1)) {
            if let Some(active_note) = current_note.take() {
                notes.push(active_note);
            }
            current_note = Some(NoteFields {
                header: header_text.as_str().to_owned(),
                body_lines: Vec::new(),
            });
        } else if let Some(ref mut active_note) = current_note {
            active_note.body_lines.push(line.to_string());
        }
    }

    if let Some(n) = current_note {
        notes.push(n)
    }

    Ok(notes)
}

pub fn read_multiple_md(path_or_glob: impl AsRef<str>) -> crate::Result<Vec<NoteFields>> {
    let files = if let Ok(globs) = glob(path_or_glob.as_ref()) {
        globs.into_iter().collect::<Result<Vec<_>, _>>()?
    } else {
        vec![expanduser(path_or_glob.as_ref())?]
    };

    let all_notes = files
        .into_iter()
        .map(read_md_file)
        .collect::<Result<HashSet<_>, _>>()?
        .into_iter()
        .flatten()
        .collect_vec();

    Ok(all_notes)
}

pub fn read_config(config_path: impl AsRef<Path>) -> crate::Result<Vec<DeckConfig>> {
    let mut config: Config = toml::from_str(&std::fs::read_to_string(config_path.as_ref())?)?;

    let deck_configs = if !config.all.type_in_prefixes.is_empty() {
        config
            .decks
            .into_iter()
            .map(|d| DeckConfig {
                type_in_prefixes: d
                    .type_in_prefixes
                    .into_iter()
                    .chain(config.all.type_in_prefixes.iter().cloned())
                    .sorted()
                    .dedup()
                    .collect(),
                ..d
            })
            .collect()
    } else {
        config.decks
    };

    if deck_configs.iter().filter(|d| d.is_default).count() != 1 {
        Err("Exactly one deck must have the 'is_default' field set to true".into())
    } else {
        Ok(deck_configs)
    }
}

fn timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time has failed")
        .as_millis()
}

pub fn make_deck_name(deck_name: impl AsRef<str>) -> String {
    format!(
        "{}_{}.apkg",
        timestamp(),
        sanitize_filename::sanitize(deck_name.as_ref())
    )
}
