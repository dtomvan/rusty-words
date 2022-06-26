#![feature(path_file_prefix, option_result_contains)]

use std::{borrow::Cow, convert::TryFrom, fs::File, io::Write};

use clap::Parser;
use color_eyre::{
    eyre::{eyre, Context},
    Help, Result,
};
use itertools::Itertools;

use cli::{GCArgs, ImportArgs, ListArgs, RmArgs};
use model::{PrimitiveWordsList, WordsIndex, WordsList, WordsMeta};
use paths::{index_file, root_dir, words_file};
use ron::ser::PrettyConfig;

mod cli;
mod model;
mod paths;

fn main() -> Result<()> {
    let args = cli::Cli::parse();
    color_eyre::install()?;
    let root_dir = root_dir()?;
    std::fs::create_dir_all(&root_dir)?;

    let index_filename = index_file()?;
    let mut index_file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(&index_filename)?;

    let mut index = match ron::de::from_reader(&mut index_file) {
        Ok(index) => index,
        _ if index_file.metadata()?.len() == 0 => WordsIndex::default(),
        Err(e) => {
            return Err(eyre!(
                "There is an error in the index file which cannot be resolved. Exiting."
            )
            .with_error(|| e));
        }
    };

    match args.command {
        cli::Command::Import(ImportArgs {
            filename,
            term_lang,
            def_lang,
            dir,
        }) => {
            let data = std::fs::read_to_string(&filename)?;
            let parsed = PrimitiveWordsList::try_from(data.as_str())
                .with_context(|| format!("while trying to import {}", filename.display()))?;

            let name = filename.file_prefix();
            let name = if let Some(name) = name {
                name.to_string_lossy()
            } else {
                let mut name = String::new();
                print!("What name do you want to give the words list?");
                std::io::stdin().read_line(&mut name)?;
                Cow::Owned(name)
            };

            let list = WordsList::from(parsed);
            let meta = WordsMeta::new(name.to_string(), term_lang, def_lang, dir);
            let words_file = words_file(&meta.uuid)?;
            index.lists.push(meta);

            let ser = ron::to_string(&list)?;
            write!(
                &mut File::options()
                    .write(true)
                    .create_new(true)
                    .open(words_file)?,
                "{ser}"
            )?;
        }
        cli::Command::Show { ids } => {
            for id in ids {
                let WordsMeta {
                    name,
                    last_modified,
                    created_at,
                    terms,
                    definition,
                    uuid,
                    folder,
                } = index.get(id)?;
                println!("Name: {}", name);
                if let Some(ref term) = terms {
                    println!("Terms: {term}");
                }
                if let Some(ref definition) = definition {
                    println!("Definitions: {definition}");
                }
                println!(
                    "Created at: {}\nLast modified at: {}",
                    last_modified, created_at
                );
                if let Some(ref folder) = folder {
                    println!("Folder: {}", folder.display());
                }
                println!("UUID: {uuid}");
            }
            return Ok(());
        }
        cli::Command::Ls(ListArgs { language }) => {
            let map = index
                .lists
                .into_iter()
                .enumerate()
                .into_group_map_by(|(_, x)| x.folder.to_owned());

            for (folder, lists) in map.into_iter().sorted_by(|(x, _), (y, _)| x.cmp(y)) {
                println!(
                    "{}:",
                    folder
                        .map(|x| x.to_string_lossy().to_string())
                        .unwrap_or_else(|| String::from("No folder"))
                );
                let mut seen_first = false;

                for (id, list) in lists.into_iter().filter(|(_, list)| {
                    if let Some(ref language) = language {
                        list.terms.contains(language) || list.definition.contains(language)
                    } else {
                        true
                    }
                }) {
                    seen_first |= true;
                    println!("{}. {}", id + 1, list.name);
                }

                if !seen_first {
                    println!("No lists...");
                }
            }
            return Ok(());
        }
        cli::Command::Rm(RmArgs { mut ids, force }) => {
            ids.sort_unstable();
            ids.dedup();
            ids.reverse();
            for id in ids {
                let mut result = || -> Result<()> {
                    let meta = index.get(id)?;
                    let words_file = words_file(&meta.uuid)?;
                    index.remove(id)?;
                    if let Err(e) = std::fs::remove_file(words_file) {
                        eprintln!(
                            "Error while removing list `{}` (list ID {}) is not important so it is ignored.",
                            id,
                            e
                        );
                    }
                    Ok(())
                };
                match result() {
                    Err(e) if !force => {
                        return Err(e);
                    }
                    _ => (),
                }
            }
        }
        cli::Command::GarbageCollect(GCArgs { dry_run }) => {
            // TODO: Also remove entries in the index that do not exist
            let patterns = index
                .lists
                .iter()
                .filter_map(|x| words_file(&x.uuid).ok())
                .collect_vec();
            for file in std::fs::read_dir(root_dir)?
                .filter_map(|entry| entry.ok().map(|ok| ok.path()))
                .filter(|path| {
                    path.extension().contains(&mut "ron")
                        && path.is_file()
                        && !path.ends_with("index.ron")
                        && !patterns.contains(&path)
                })
            {
                if !dry_run {
                    println!("Removing {}...", file.display());
                    std::fs::remove_file(file)?;
                } else {
                    println!("Would remove {}", file.display());
                }
            }
        }
        _ => todo!(),
    }

    let ser = ron::ser::to_string_pretty(&index, PrettyConfig::default())?;
    let mut index_file = File::create(index_filename)?;
    write!(&mut index_file, "{ser}")?;

    Ok(())
}
