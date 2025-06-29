use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use crate::formats::rzx;

#[derive(Parser)]
#[command(name = "rzx")]
#[command(about = "A modern, fast compression tool with .rzx format support")]
#[command(version = "0.1.0")]
#[command(author = "Nurul Fajar")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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

        /// Encryption algorithm (none, aes256gcm)
        #[arg(short, long, default_value = "none")]
        encryption: String,

        /// Password for encryption
        #[arg(short, long)]
        password: Option<String>,

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

        /// Password for decryption
        #[arg(short, long)]
        password: Option<String>,

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

        /// Password for decryption
        #[arg(short, long)]
        password: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Test archive integrity
    Test {
        /// Archive file to test
        archive: PathBuf,

        /// Password for decryption
        #[arg(short, long)]
        password: Option<String>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show information about RZX format
    Info {
        /// Archive file to analyze
        archive: PathBuf,

        /// Password for decryption
        #[arg(short, long)]
        password: Option<String>,
    },
}

pub fn create_archive(
    output: PathBuf,
    inputs: Vec<PathBuf>,
    level: u8,
    algorithm: String,
    encryption: String,
    password: Option<String>,
    exclude: Vec<String>,
) -> Result<()> {
    let encryption_algorithm = crate::core::EncryptionAlgorithm::from_str(&encryption)
        .ok_or_else(|| anyhow::anyhow!("Unsupported encryption algorithm"))?;
    Ok(rzx::create_archive(
        &output,
        &inputs,
        level,
        &algorithm,
        encryption_algorithm,
        password.as_deref(),
        &exclude,
    )?)
}

pub fn extract_archive(archive: PathBuf, output: PathBuf, password: Option<String>, force: bool) -> Result<()> {
    Ok(rzx::extract_archive(&archive, &output, password.as_deref(), force)?)
}

pub fn list_archive(archive: PathBuf, detailed: bool, password: Option<String>) -> Result<()> {
    Ok(rzx::list_archive(&archive, detailed, password.as_deref())?)
}

pub fn test_archive(archive: PathBuf, password: Option<String>) -> Result<()> {
    Ok(rzx::test_archive(&archive, password.as_deref())?)
}

pub fn show_archive_info(archive: PathBuf, password: Option<String>) -> Result<()> {
    Ok(rzx::show_archive_info(&archive, password.as_deref())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_create_command() {
        let cli = Cli::try_parse_from(&[
            "rzx",
            "create",
            "-o", "archive.rzx",
            "-l", "9",
            "-a", "lzma",
            "-e", "aes256gcm",
            "-p", "password123",
            "-x", "*.log",
            "-x", "temp/",
            "input1.txt",
            "input2.txt",
        ]).unwrap();

        match cli.command {
            Commands::Create {
                output,
                inputs,
                level,
                algorithm,
                encryption,
                password,
                exclude,
                verbose,
            } => {
                assert_eq!(output, PathBuf::from("archive.rzx"));
                assert_eq!(inputs, vec![PathBuf::from("input1.txt"), PathBuf::from("input2.txt")]);
                assert_eq!(level, 9);
                assert_eq!(algorithm, "lzma");
                assert_eq!(encryption, "aes256gcm");
                assert_eq!(password, Some("password123".to_string()));
                assert_eq!(exclude, vec!["*.log".to_string(), "temp/".to_string()]);
                assert_eq!(verbose, false);
            }
            _ => panic!("Unexpected command"),
        }
    }

    #[test]
    fn test_extract_command() {
        let cli = Cli::try_parse_from(&[
            "rzx",
            "extract",
            "my_archive.rzx",
            "-o", "output_dir",
            "-p", "secret",
            "--force",
        ]).unwrap();

        match cli.command {
            Commands::Extract {
                archive,
                output,
                password,
                force,
                verbose,
            } => {
                assert_eq!(archive, PathBuf::from("my_archive.rzx"));
                assert_eq!(output, PathBuf::from("output_dir"));
                assert_eq!(password, Some("secret".to_string()));
                assert_eq!(force, true);
                assert_eq!(verbose, false);
            }
            _ => panic!("Unexpected command"),
        }
    }

    #[test]
    fn test_list_command() {
        let cli = Cli::try_parse_from(&[
            "rzx",
            "list",
            "my_archive.rzx",
            "-p", "secret",
            "--detailed",
        ]).unwrap();

        match cli.command {
            Commands::List {
                archive,
                detailed,
                password,
            } => {
                assert_eq!(archive, PathBuf::from("my_archive.rzx"));
                assert_eq!(detailed, true);
                assert_eq!(password, Some("secret".to_string()));
            }
            _ => panic!("Unexpected command"),
        }
    }

    #[test]
    fn test_test_command() {
        let cli = Cli::try_parse_from(&[
            "rzx",
            "test",
            "my_archive.rzx",
            "-p", "secret",
        ]).unwrap();

        match cli.command {
            Commands::Test {
                archive,
                password,
                verbose,
            } => {
                assert_eq!(archive, PathBuf::from("my_archive.rzx"));
                assert_eq!(password, Some("secret".to_string()));
                assert_eq!(verbose, false);
            }
            _ => panic!("Unexpected command"),
        }
    }

    #[test]
    fn test_info_command() {
        let cli = Cli::try_parse_from(&[
            "rzx",
            "info",
            "my_archive.rzx",
            "-p", "secret",
        ]).unwrap();

        match cli.command {
            Commands::Info {
                archive,
                password,
            } => {
                assert_eq!(archive, PathBuf::from("my_archive.rzx"));
                assert_eq!(password, Some("secret".to_string()));
            }
            _ => panic!("Unexpected command"),
        }
    }
}