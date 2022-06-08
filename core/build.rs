use std::error::Error;
use std::process::Command;

fn get_commit_hash() -> Result<String, Box<dyn Error>> {
    let command = Command::new("git").arg("rev-parse").arg("HEAD").output()?;
    if command.status.success() {
        String::from_utf8(command.stdout).map_err(|e| e.into())
    } else {
        Err("Git rev-parse HEAD failed".into())
    }
}

fn get_commit_date() -> Result<String, Box<dyn Error>> {
    let command = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--format=%cd")
        .output()?;
    if command.status.success() {
        // this can be simplified after ExitStatus::exit_ok is stabillized
        String::from_utf8(command.stdout).map_err(|e| e.into())
    } else {
        Err("Git log failed".into())
    }
}

fn main() {
    match get_commit_hash() {
        Ok(hash) => println!("cargo:rustc-env=GIT_COMMIT_HASH={}", hash),
        Err(e) => {
            println!("cargo:rustc-env=GIT_COMMIT_HASH=");
            println!("cargo:warning=Error getting commit hash, error: {}", e)
        }
    }
    match get_commit_date() {
        Ok(date) => println!("cargo:rustc-env=GIT_COMMIT_DATE={}", date),
        Err(e) => {
            println!("cargo:rustc-env=GIT_COMMIT_HASH=");
            println!("cargo:warning=Error getting commit hash, error: {}", e)
        }
    }
}
