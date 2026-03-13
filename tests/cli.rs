use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("sffs").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage: sffs"));
}

#[test]
fn test_cli_current_dir() {
    let mut cmd = Command::cargo_bin("sffs").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("SUMMARY"));
}

#[test]
fn test_cli_with_path() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "Hello, world!").unwrap();

    let mut cmd = Command::cargo_bin("sffs").unwrap();
    cmd.arg(dir.path()).arg("--silent");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Total Size:"))
        .stdout(predicate::str::contains("13 B").or(predicate::str::contains("109 B")));
}

#[test]
fn test_cli_top_n() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("small.txt");
    let file2 = dir.path().join("large.txt");
    fs::write(&file1, "small").unwrap(); // 5 bytes
    fs::write(&file2, "much larger file content").unwrap(); // 24 bytes

    let mut cmd = Command::cargo_bin("sffs").unwrap();
    cmd.arg(dir.path()).arg("--top").arg("1");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TOP 1"))
        .stdout(predicate::str::contains("large.txt"));
}

#[test]
fn test_cli_silent() {
    let mut cmd = Command::cargo_bin("sffs").unwrap();
    cmd.arg("--silent");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Total Size:"));
}
