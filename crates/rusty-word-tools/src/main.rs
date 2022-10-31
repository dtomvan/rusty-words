use clap::{Parser, Subcommand};
use color_eyre::Result;

mod t2k;
pub mod wrts;

use t2k::{to_t2k, to_tsv};
use wrts::to_json;

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    match args.command {
        Commands::ToT2k {
            font_question,
            font_answer,
        } => to_t2k(font_question, font_answer)?,
        Commands::ToTsv => to_tsv()?,
        Commands::ToJson {
            subject_id,
            flipped,
            title,
            desc,
        } => to_json(subject_id, flipped, title, desc)?,
    };

    Ok(())
}

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    /// Convert tsv to t2k
    ToT2k {
        font_question: Option<String>,
        font_answer: Option<String>,
    },
    ToTsv,
    ToJson {
        subject_id: usize,
        flipped: bool,
        title: String,
        desc: Option<String>,
    },
}

#[derive(Debug, Clone, Parser)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}
