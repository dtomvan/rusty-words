use std::{
    collections::HashMap,
    io::{BufRead, BufReader, stdin, stdout},
};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use once_cell::sync::Lazy;

static LOCALES: Lazy<HashMap<usize, &str>> =
    Lazy::new(|| HashMap::from([(3, "fr-FR"), (6, "la-VA"), (7, "el-GR")]));

/// https://api.wrts.nl/api/v3/lists
#[derive(Serialize, Deserialize)]
pub struct Lists {
    list: List,
}

#[derive(Serialize, Deserialize)]
pub struct List {
    title: String,
    description: Option<String>,
    status: Status,
    words_collection: Vec<Word>,
    shared: bool,
    subject_id: usize,
    locales: (String, String),
}

#[derive(Serialize, Deserialize)]
pub struct Word {
    id: usize,
    words: (String, String),
    image_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Draft,
    Active,
}

pub fn to_json(
    subject_id: usize,
    flipped: bool,
    title: String,
    description: Option<String>,
) -> Result<()> {
    let reader = BufReader::new(stdin());
    let words = reader
        .lines()
        .enumerate()
        .map(|(i, x)| {
            let (term, def) = x
                .as_deref()
                .unwrap()
                .split_once('\t')
                .expect("Should be `term<tab>definition`");
            Word {
                id: i + 1,
                words: (term.to_string(), def.to_string()),
                image_url: None,
            }
        })
        .collect();

    let locale = LOCALES[&subject_id];
    let locales = if flipped {
        (String::from(locale), String::from("nl-NL"))
    } else {
        (String::from("nl-NL"), String::from(locale))
    };
    let lists = Lists {
        list: List {
            title,
            description,
            status: Status::Active,
            words_collection: words,
            shared: true,
            subject_id,
            locales,
        },
    };
    serde_json::to_writer(stdout().lock(), &lists)?;
    Ok(())
}
