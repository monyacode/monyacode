#![allow(clippy::disallowed_methods, reason = "build scripts are exempt")]
use std::process::Command;

fn main() {
    println!("cargo:rustc-env=TARGET={}", std::env::var("TARGET").unwrap());

    // Populate git sha environment variable if git is available
    println!("cargo:rerun-if-changed=../../.git/logs/HEAD");
    if let Some(output) = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let git_sha = String::from_utf8_lossy(&output.stdout);
        let git_sha = git_sha.trim();
        println!("cargo:rustc-env=MONYACODE_COMMIT_SHA={git_sha}");
    }

    if let Some(output) = Command::new("git")
        .args(["describe", "--tags"])
        .output()
        .ok()
        .filter(|output| output.status.success())
    {
        let git_name = String::from_utf8_lossy(&output.stdout);
        let git_name = git_name.trim();
        println!("cargo:rustc-env=MONYACODE_COMMIT_NAME={git_name}");
    }
}
