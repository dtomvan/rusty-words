use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use rusty_words_common::judgement::TryMethod;
// HACK: ImportArgs is now in common, but the rest of the args parsing is not because ImportArgs is
// consumed by `common::import_list`.
use rusty_words_common::model::{ImportArgs, WordsDirection};

#[derive(Parser, Debug, Clone)]
#[clap(about, author, version)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: self::Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Create a new words list
    New(NewArgs),
    /// Import an existing words list (tsv or ron)
    Import(ImportArgs),
    /// List all existing words lists
    Ls(ListArgs),
    /// Show all information about a words list by ID
    Show(ShowArgs),
    /// Edit an existing words list by ID
    Edit { id: usize },
    /// Learn word list by ID
    Try(TryArgs),
    /// Delete word list by ID
    Rm(RmArgs),
    /// Removes all words lists in the store that are not currently in the index
    GarbageCollect(GCArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ShowArgs {
    pub ids: Vec<usize>,
    #[clap(short, long)]
    pub porcelain: bool,
}

#[derive(Args, Debug, Clone)]
pub struct GCArgs {
    #[clap(short, long)]
    pub dry_run: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RmArgs {
    pub ids: Vec<usize>,
    #[clap(short, long)]
    pub force: bool,
}

#[derive(Args, Debug, Clone)]
pub struct NewArgs {
    pub name: String,
    pub term_lang: String,
    pub def_lang: String,
    #[clap(short, long)]
    pub dir: Option<PathBuf>,
    #[clap(value_enum, long)]
    pub direction: Option<WordsDirection>,
}

#[derive(Args, Debug, Clone)]
pub struct TryArgs {
    pub id: usize,
    #[clap(value_enum)]
    pub method: TryMethod,
    #[clap(value_enum, short, long)]
    pub direction: Option<WordsDirection>,
    #[clap(short, long)]
    pub shuffle: bool,
    #[clap(short, long)]
    pub reset: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ListArgs {
    /// Optional filter by language
    pub filter: Option<String>,
}
