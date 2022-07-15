use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(
    author,
    version,
    long_about = "Parse EU’s DGT-Translation Memory, distributed as a collection of TMX files ZIP archives, and save the multilingual parallel texts into other output formats."
)]
#[clap(author = "Paweł Malinowski")]
#[clap(about = "Parse and transform the DGT-TM (translation memory).")]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,

    /// Path to directory containing a flat collection of ZIP files
    #[clap(short, long)]
    #[clap(display_order = 1)]
    pub input_dir: PathBuf,

    /// Languages that should be included in the output
    #[clap(short)]
    #[clap(display_order = 2)]
    pub langs: Option<Vec<String>>,

    /// Only include translation units where each of the specified languages is present
    #[clap(short, long)]
    #[clap(display_order = 3)]
    #[clap(requires = "langs")]
    pub require_each_lang: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[clap(display_order = 1)]
    /// Save translation units in an SQLite database
    Sqlite {
        /// Output file
        #[clap(short, long = "output")]
        output_file: String,
    },
}
