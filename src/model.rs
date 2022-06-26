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

use std::{borrow::Cow, collections::HashMap, convert::TryFrom, path::PathBuf};

use chrono::{DateTime, Utc};
use clap::clap_derive::ArgEnum;
use color_eyre::{
    eyre::{eyre, Context},
    Help, Report, Result,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

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
}

#[serde_as]
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WordsMeta {
    pub name: String,
    #[serde_as(as = "DisplayFromStr")]
    pub uuid: Uuid,
    pub terms: Option<String>,
    pub definition: Option<String>,
    #[serde_as(as = "DisplayFromStr")]
    pub created_at: DateTime<Utc>,
    #[serde_as(as = "DisplayFromStr")]
    pub last_modified: DateTime<Utc>,
    pub folder: Option<PathBuf>,
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
            terms,
            definition,
            folder,
            uuid,
            created_at,
            last_modified: created_at,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WordsList<'a>(Vec<WordsEntry<'a>>);

impl<'a> From<PrimitiveWordsList<'a>> for WordsList<'a> {
    fn from(input: PrimitiveWordsList<'a>) -> WordsList<'a> {
        WordsList(
            input
                .0
                .into_iter()
                .map(|(term, definitions)| WordsEntry {
                    term,
                    definitions,
                    direction: WordsDirection::TD,
                    times_answered_correctly: 0,
                })
                .collect(),
        )
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct WordsEntry<'a> {
    term: String,
    definitions: Vec<Cow<'a, str>>,
    direction: WordsDirection,
    times_answered_correctly: usize,
}

#[derive(ArgEnum, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
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
            let values = split
                .next()
                .ok_or_else(|| eyre!("Couldn't parse line number {n}: No such key"))?;
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
