use clap::{Parser, Subcommand};
use color_eyre::Result;

mod t2k;

use t2k::{to_t2k, to_tsv};

fn main() -> Result<()> {
    color_eyre::install()?;
    let args = Args::parse();

    match args.command {
        Commands::ToT2k {
            font_question,
            font_answer,
        } => {
            to_t2k(font_question, font_answer)?;
        }
        Commands::ToTsv => {
            to_tsv()?;
        },
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
}

#[derive(Debug, Clone, Parser)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}
