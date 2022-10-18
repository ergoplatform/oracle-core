use std::error::Error;
use std::process::Command;

fn run_command(bin: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let command = Command::new(bin).args(args).output()?;
    if command.status.success() {
        String::from_utf8(command.stdout).map_err(|e| e.into())
    } else {
        Err(format!("{} {:?} failed", bin, args).into())
    }
}

fn main() {
    println!("cargo:rustc-env=GIT_COMMIT_INFO=");
    if let Ok(git_path) = run_command("git", &["rev-parse", "--show-cdup"]) {
        // Check whether .git repository belongs to oracle-core, since GitHub releases do not include .git
        if git_path.trim_end() == "../" {
            let mut commit_hash = String::new();
            let mut commit_date = String::new();
            match run_command("git", &["rev-parse", "HEAD"]) {
                Ok(hash) => commit_hash = hash,
                Err(e) => {
                    println!("cargo:warning=Error getting commit hash, error: {}", e)
                }
            }
            match run_command("git", &["log", "-1", "--format=%cd"]) {
                Ok(date) => commit_date = date,
                Err(e) => {
                    println!("cargo:warning=Error getting commit hash, error: {}", e)
                }
            }
            println!("cargo:rustc-env=GIT_COMMIT_HASH={}", &commit_hash[0..7]);
            println!("cargo:rustc-env=GIT_COMMIT_DATE={}", commit_date);
        }
    };
}
