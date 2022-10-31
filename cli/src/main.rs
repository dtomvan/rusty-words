#![feature(
    option_get_or_insert_default,
    path_file_prefix,
    option_result_contains,
    drain_filter,
    int_roundings
)]

use std::{fs::File, io::Write, process::Command};

use clap::Parser;
use color_eyre::{eyre::eyre, Help, Result};
use itertools::Itertools;

use args::{GCArgs, ImportArgs, ListArgs, NewArgs, RmArgs, ShowArgs, TryArgs};
use ron::ser::PrettyConfig;
use rusty_words_common::model::{WordsDirection, WordsIndex, WordsList};
use rusty_words_common::paths::{index_file, root_dir, words_file_exists};

mod args;
mod tui;

fn main() -> Result<()> {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let args = args::Cli::parse();
    color_eyre::install()?;
    let root_dir = root_dir()?;
    std::fs::create_dir_all(&root_dir)?;

    // We always load and save our index at the program's entry point, and then we apply changes to
    // the index throughout.
    //
    // We always save our changes, unless we do an early return, implying there is no need to save
    // the store.
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
        args::Command::Import(ImportArgs {
            filename,
            term_lang,
            def_lang,
            dir,
            direction,
        }) => {
            let data = std::fs::read_to_string(&filename)?;
            let name = filename.file_prefix();
            let name = if let Some(name) = name {
                name.to_string_lossy().to_string()
            } else {
                let mut name = String::new();
                print!("What name do you want to give the words list? -> ");
                std::io::stdin().read_line(&mut name)?;
                name
            };
            let id = index.import_list(
                name.clone(),
                &data,
                &filename,
                term_lang,
                def_lang,
                dir,
                direction,
            )?;
            println!(
                "Successfully imported words list `{}` from `{}` with ID {}.",
                name,
                filename.display(),
                id
            );
        }
        args::Command::Show(ShowArgs { ids, porcelain }) => {
            for id in ids {
                let meta = index.get(id)?;
                let words_file = words_file_exists(&root_dir, &meta.uuid)?;
                let mut words_file = File::open(words_file)?;
                let words: WordsList = ron::de::from_reader(&mut words_file)?;
                if porcelain {
                    println!("{meta:#}\n{words:#}");
                } else {
                    println!("{meta}\n{words}");
                }
            }
            return Ok(());
        }
        args::Command::Ls(ListArgs { filter }) => {
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
                    filter.is_none() || filter == list.terms.0 || filter == list.definition.0
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
        args::Command::Rm(RmArgs { mut ids, force }) => {
            ids.sort_unstable();
            ids.dedup();
            ids.reverse();
            for id in ids {
                let mut result = || -> Result<()> {
                    let meta = index.get(id)?;
                    let words_file = words_file_exists(&root_dir, &meta.uuid)?;
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
        args::Command::GarbageCollect(GCArgs { dry_run }) => {
            let mut patterns = index
                .lists
                .iter()
                .enumerate()
                .map(|(i, x)| (i, x.uuid, words_file_exists(&root_dir, &x.uuid)))
                .collect_vec();
            let mut not_exists = patterns
                .drain_filter(|(_, _, x)| x.is_err())
                .map(|(i, x, _)| (i, x))
                .collect_vec();
            not_exists.sort_by_key(|(i, _)| *i);
            not_exists.reverse();
            for (i, not_exists) in not_exists {
                if !dry_run {
                    println!(
                        "Removing {} from index (id: {}) (file doesn't exist)",
                        not_exists,
                        i + 1
                    );
                    // 1-indexed
                    index.remove(i + 1)?;
                } else {
                    println!(
                        "Would remove {} from index (file doesn't exist)",
                        not_exists
                    );
                }
            }
            let patterns = patterns
                .into_iter()
                .map(|(_, _, x)| x.unwrap())
                .collect_vec();
            for file in std::fs::read_dir(root_dir)?
                .filter_map(|entry| entry.ok().map(|ok| ok.path()))
                .filter(|path| {
                    path.extension().contains(&"ron")
                        && path.is_file()
                        && !path.ends_with("index.ron")
                        && !patterns.contains(path)
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
        args::Command::New(NewArgs {
            name,
            term_lang,
            def_lang,
            dir,
            direction,
        }) => {
            let path = root_dir.join("temp.tsv");
            drop(File::create(&path)?);
            let editor = std::env::var_os("EDITOR");
            let found = editor.is_some();
            let editor = editor.unwrap_or(if cfg!(windows) {
                "notepad".into()
            } else if cfg!(darwin) {
                "/Applications/TextEdit.app/Contents/MacOS/TextEdit".into()
            } else {
                // Let's hope you have vim in this case
                "vim".into()
            });
            Command::new(&editor)
                .arg(&path)
                .spawn()
                .with_note(|| "while trying to spawn your editor")
                .with_note(|| format!("tried editor {}", editor.to_string_lossy()))
                .with_note(|| {
                    if found {
                        "tried because $EDITOR was set"
                    } else {
                        "tried because $EDITOR wasn't set (default value is notepad/TextEdit/vim)"
                    }
                })
                .with_suggestion(|| "Try setting $EDITOR correctly (or installing vim)")?
                .wait()?;
            let data = std::fs::read_to_string(&path)?;
            let id = index.import_list(
                name,
                &data,
                &path,
                Some(term_lang),
                Some(def_lang),
                dir,
                direction,
            )?;
            println!("Successfully created list {id}.");
        }
        args::Command::Try(TryArgs {
            id,
            method,
            direction,
            shuffle,
            reset,
        }) => {
            tui::try_list(
                &mut index,
                id,
                method,
                direction.unwrap_or(WordsDirection::Auto),
                shuffle,
                reset,
            )?;
        }
        _ => todo!(),
    }

    let ser = ron::ser::to_string_pretty(&index, PrettyConfig::default())?;
    let mut index_file = File::create(index_filename)?;
    write!(&mut index_file, "{ser}")?;

    Ok(())
}
