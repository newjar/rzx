use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_create_and_extract_single_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempdir()?;
    let output_dir = temp_dir.path().join("output");
    let archive_path = temp_dir.path().join("test.rzx");
    let test_file_path = temp_dir.path().join("test_file.txt");
    let file_content = "Hello, single file!";
    std::fs::write(&test_file_path, file_content)?;

    // Test `create` command
    Command::cargo_bin("rzx")?
        .arg("create")
        .arg("--output")
        .arg(&archive_path)
        .arg(&test_file_path)
        .assert()
        .success();

    // Test `extract` command
    std::fs::create_dir(&output_dir)?;
    Command::cargo_bin("rzx")?
        .arg("extract")
        .arg(&archive_path)
        .arg("-o")
        .arg(&output_dir)
        .assert()
        .success();

    // Verify extracted file
    let extracted_file_path = output_dir.join("test_file.txt");
    assert!(extracted_file_path.exists());
    assert_eq!(
        std::fs::read_to_string(&extracted_file_path)?,
        file_content
    );

    Ok(())
}
