use anyhow::{Ok, Result};
use clap::Parser;
use rzx::cli;

fn main() -> Result<()> {
    env_logger::init();

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Create { output, inputs, level, algorithm, encryption, password, exclude, verbose: _ } => {
            cli::create_archive(output, inputs, level, algorithm, encryption, password, exclude)?;
        }
        cli::Commands::Extract { archive, output, force, password, verbose: _ } => {
            cli::extract_archive(archive, output, password, force)?;
        }
        cli::Commands::List { archive, detailed, password } => {
            cli::list_archive(archive, detailed, password)?;
        }
        cli::Commands::Test { archive, password, verbose: _ } => {
            cli::test_archive(archive, password)?;
        }
        cli::Commands::Info { archive, password } => {
            cli::show_archive_info(archive, password)?;
        }
    }

    Ok(())
}
