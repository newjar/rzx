use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod cli;
mod compression;
mod core;
mod formats;
mod utils;

#[derive(Parser)]
#[command(name = "rzx")]
#[command(about = "A modern, fast compression tool with .rzx format support")]
#[command(version = "0.1.0")]
#[command(author = "Nurul Fajar")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new RZX archive
    Create {
        /// Output archive file
        #[arg(short, long)]
        output: PathBuf,

        /// Files and directories to compress
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        /// Compression level (1-9)
        #[arg(short, long, default_value = "6")]
        level: u8,

        /// Compression algorithm (deflate, lzma, zstd)
        #[arg(short, long, default_value = "deflate")]
        algorithm: String,

        /// Exclude pattern
        #[arg(short = 'x', long)]
        exclude: Vec<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Extract files from RZX archive
    Extract {
        /// Archive file to extract
        archive: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Overwrite existing files
        #[arg(short, long)]
        force: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List contents of RZX archive
    List {
        /// Archive file to list
        archive: PathBuf,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Test archive integrity
    Test {
        /// Archive file to test
        archive: PathBuf,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show information about RZX format
    Info {
        /// Archive file to analyze
        archive: PathBuf,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Create {
            output,
            inputs,
            level,
            algorithm,
            exclude,
            verbose,
        } => {
            println!("Creating archive: {:?}", output);
            if verbose {
                println!("Inputs: {:?}", inputs);
                println!("Level: {}, Algorithm: {}", level, algorithm)
            }

            cli::create_archive(output, inputs, level, algorithm, exclude, verbose)?;
        }

        Commands::Extract {
            archive,
            output,
            force,
            verbose,
        } => {
            println!("Extracting archive: {:?} to {:?}", archive, output);
            cli::extract_archive(archive, output, force, verbose)?;
        }

        Commands::List { archive, detailed } => {
            println!("Listing contents of: {:?}", archive);
            cli::list_archive(archive, detailed)?;
        }

        Commands::Info { archive } => {
            println!("Archive info: {:?}", archive);
            cli::show_archive_info(archive)?;
        }

        Commands::Test { archive, verbose } => {
            println!("Testing archive: {:?}", archive);
            // TODO: Implement test functionality
            cli::test_archive(archive, verbose)?;
        }
    }

    Ok(())
}
