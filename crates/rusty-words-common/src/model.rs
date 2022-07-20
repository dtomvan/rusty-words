//! Data model:
//! ~/.local/share/rusty-words/index.ron -> Index file containing an array of metadata
//!     The index is the ID.
//!     Every entry in this array has data like:
//!     - Name of list
//!     - Path to words list file (located in ~/.local/share/rusty-words/UUID.ron)
//!         -> Stores term, definition, times correctly answered for every word.
//!         -> Tsv files can be imported and exported (losing all data except term and definition)
//!     - Language for term and definition
//!     - When created/modified
//!     - How to check the user if they got a word correct
//!     - The (fake) folder the list is in
//! - The index file also contains the user's native language (for filtering)
//!     Ex: The user's native lanuage is "nl" and they search for "en",
//!         they want the lists that practice nl->en or en->nl.
//!     Ex: The user's native lanuage is "de" and they search for "en",
//!         they are probably not looking for en->nl or nl->en, so we sort that later in the list.

use std::{
    borrow::Cow,
    collections::HashMap,
    convert::TryFrom,
    fmt::{Debug, Display},
    fs::File,
    io::Write,
    ops::BitAnd,
    path::{Path, PathBuf},
};

use aho_corasick::AhoCorasick;
use chrono::{DateTime, Utc};
use clap::clap_derive::ArgEnum;
use color_eyre::{
    eyre::{eyre, Context},
    Help, Report, Result,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use tabled::{Style, Table, Tabled};
use uuid::Uuid;

use crate::paths::new_words_file;

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Default)]
pub struct WordsIndex {
    pub lists: Vec<WordsMeta>,
}

impl WordsIndex {
    fn _underflow() -> Report {
        eyre!("Integer underflow. ID's are 1-indexed.")
    }
    fn _could_not_find(id: usize) -> Report {
        eyre!("Could not find that list by ID {id}")
    }
    pub fn get(&self, id: usize) -> Result<&WordsMeta> {
        let meta = self
            .lists
            .get(
                id.checked_sub(1)
                    .ok_or_else(Self::_underflow)
                    .with_context(|| "While getting a list.")?,
            )
            .ok_or_else(|| {
                Self::_could_not_find(id).with_note(|| "Occured while getting a list.")
            })?;
        Ok(meta)
    }
    pub fn remove(&mut self, id: usize) -> Result<WordsMeta> {
        if id > self.lists.len() {
            return Err(Self::_could_not_find(id).with_note(|| "Occured while deleting a list."));
        }
        let meta = self
            .lists
            .remove(id.checked_sub(1).ok_or_else(Self::_underflow)?);
        Ok(meta)
    }

    /// Returns the ID of the new entry
    pub fn import_list(
        &mut self,
        name: String,
        data: &str,
        filename: &Path,
        term_lang: Option<String>,
        def_lang: Option<String>,
        dir: Option<PathBuf>,
    ) -> Result<usize> {
        let parsed = PrimitiveWordsList::try_from(data)
            .with_context(|| format!("while trying to import {}", filename.display()))?;

        let list = WordsList::from(parsed);
        let meta = WordsMeta::new(name, term_lang, def_lang, dir);
        let words_file = new_words_file(&meta.uuid)?;
        self.lists.push(meta);

        let ser = ron::ser::to_string_pretty(&list, PrettyConfig::default())?;
        write!(
            &mut File::options()
                .write(true)
                .create_new(true)
                .open(words_file)?,
            "{ser}"
        )?;

        Ok(self.lists.len())
    }
}

#[serde_as]
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WordsMeta {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    pub uuid: Uuid,
    pub terms: Language,
    pub definition: Language,
    #[serde_as(as = "DisplayFromStr")]
    pub created_at: DateTime<Utc>,
    #[serde_as(as = "DisplayFromStr")]
    pub last_modified: DateTime<Utc>,
    pub folder: Option<PathBuf>,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Language(pub Option<String>);

impl Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(ref s) => write!(f, "{}", format_language_code(&s)),
            None => write!(f, "not set"),
        }
    }
}

impl Debug for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                Some(ref s) => &s,
                None => "null",
            }
        )
    }
}

pub fn format_language_code(haystack: impl AsRef<str>) -> String {
    let ac = AhoCorasick::new(super::lang_codes::LANG_SEARCHES);
    ac.replace_all(
        &haystack.as_ref().to_ascii_lowercase(),
        super::lang_codes::LANG_REPLACES,
    )
}

impl Display for WordsMeta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let WordsMeta {
            name,
            last_modified,
            created_at,
            terms,
            definition,
            uuid,
            folder,
        } = self;
        if f.alternate() {
            writeln!(
                f,
                "name\t{name}\nterm_lang\t{terms:?}\ndef_lang\t{definition:?}\ncreated_at\t{}\nlast_modified\t{}\nfolder\t{}\nuuid\t{}",
                last_modified,
                created_at,
                folder.clone().unwrap_or_else(|| PathBuf::from("null")).display(),
                uuid,
            )?;
        } else {
            writeln!(f, "Name: {name}")?;
            if let Some(ref term) = terms.0 {
                writeln!(f, "Terms: {}", format_language_code(term))?;
            }
            if let Some(ref definition) = definition.0 {
                writeln!(f, "Definitions: {}", format_language_code(definition))?;
            }
            writeln!(
                f,
                "Created at: {}\nLast modified at: {}",
                last_modified, created_at
            )?;
            if let Some(ref folder) = folder {
                writeln!(f, "Folder: {}", folder.display())?;
            }
            writeln!(f, "UUID: {uuid}")?;
        }
        Ok(())
    }
}

impl WordsMeta {
    pub fn new(
        name: String,
        terms: Option<String>,
        definition: Option<String>,
        folder: Option<PathBuf>,
    ) -> Self {
        let uuid = uuid::Builder::from_random_bytes(rand::random()).into_uuid();
        let created_at = chrono::Utc::now();

        Self {
            name,
            terms: Language(terms),
            definition: Language(definition),
            folder,
            uuid,
            created_at,
            last_modified: created_at,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
// TODO: Keep track of progress (e.g. continue where you left off)
// this can be done by keeping track of the last made shuffle and of which n-value we are at.
// we do not need to keep track of the rotation buffer, as it will be semi-consistent.
pub struct WordsList<'a>(pub Vec<WordsEntry<'a>>);

impl Display for WordsList<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            for entry in &self.0 {
                writeln!(
                    f,
                    "{}\t{}\t{:?}\t{}",
                    entry.terms.join(","),
                    entry.definitions.join(","),
                    entry.direction,
                    entry.times_answered_correctly
                )?;
            }
        } else {
            let table = Table::new(self.0.iter().map(|x| PrintableWordsEntry::from(x.clone())))
                .with(Style::modern().header_off().horizontal_off());
            writeln!(f, "{table}")?;
        };
        Ok(())
    }
}

impl<'a> From<PrimitiveWordsList<'a>> for WordsList<'a> {
    fn from(input: PrimitiveWordsList<'a>) -> WordsList<'a> {
        WordsList(
            input
                .0
                .into_iter()
                .map(|(term, definitions)| WordsEntry {
                    terms: vec![Cow::Owned(term)],
                    definitions,
                    direction: WordsDirection::TD,
                    times_answered_correctly: 0,
                })
                .collect(),
        )
    }
}

#[derive(Clone, Tabled)]
pub struct PrintableWordsEntry {
    term: String,
    definitions: String,
    direction: WordsDirection,
    #[tabled(rename = "times answered correctly")]
    times_correct: usize,
}

impl From<WordsEntry<'_>> for PrintableWordsEntry {
    fn from(w: WordsEntry) -> Self {
        Self {
            term: w.terms.join(", "),
            definitions: w.definitions.join(", "),
            direction: w.direction,
            times_correct: w.times_answered_correctly,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WordsEntry<'a> {
    pub terms: Vec<Cow<'a, str>>,
    pub definitions: Vec<Cow<'a, str>>,
    pub direction: WordsDirection,
    pub times_answered_correctly: usize,
}

#[derive(ArgEnum, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Tabled)]
pub enum WordsDirection {
    /// Automatic, determined by list
    Auto,
    /// Term => definition
    #[default]
    TD,
    /// Definition => term
    DT,
    /// Both ways
    Both,
}

impl BitAnd<Self> for WordsDirection {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        use WordsDirection::*;
        match (self, rhs) {
            (Auto, e) | (e, Auto) => e,
            (TD, DT) => DT,
            (DT, TD) => DT,
            (TD, TD) => TD,
            // Not sure
            (DT, DT) => DT,
            (_, Both) | (Both, _) => Both,
        }
    }
}

impl Display for WordsDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WordsDirection::Auto => "automatic",
                WordsDirection::TD => "term -> definition",
                WordsDirection::DT => "definition -> term",
                WordsDirection::Both => "both",
            }
        )
    }
}

/// File format: KEY<tab/equals>VALUE1<comma/slash>VALUE2
/// Values are always trimmed when testing for correctness.
/// Values can optionally be checked for
// Ex. (nl -> en): "bank	sofa, bank"
pub struct PrimitiveWordsList<'a>(HashMap<String, Vec<Cow<'a, str>>>);

impl<'a> TryFrom<&'a str> for PrimitiveWordsList<'a> {
    type Error = color_eyre::Report;

    fn try_from(s: &'a str) -> Result<Self> {
        let mut map = HashMap::with_capacity(s.split_terminator('\n').count());
        for (n, line) in s.split_terminator('\n').enumerate() {
            let mut split = line.split(['\t', '=']);
            let key = split.next().unwrap().trim();
            let values = split.next().ok_or_else(|| {
                eyre!(
                    "Couldn't parse line number {}: Term needs definition",
                    n + 1
                )
            })?;
            let mut values = values
                .split_terminator([',', '/'])
                .map(|x| Cow::Borrowed(x.trim()))
                .collect();
            map.entry(key.to_string())
                .or_insert_with(Vec::new)
                .append(&mut values)
        }
        Ok(Self(map))
    }
}
