use std::process::Command;
use std::time::Instant;
use std::error::Error;

fn get_commit_hash() -> Result<String, Box<dyn Error>> {
    String::from_utf8(Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()?
    .stdout).map_err(|e| e.into())
}

fn main() {
    match get_commit_hash() {
        Ok(hash) => println!("cargo:rustc-env=GIT_HASH={}", hash),
        Err(e) => println!("cargo:warning=Error getting commit hash, error: {}", e)
    }

}
