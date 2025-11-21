use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("failed to execute git command");
    let sha1 = std::str::from_utf8(&output.stdout)
        .expect("failed to parse git output as UTF-8")
        .trim();

    println!("cargo:rustc-env=APP_REVISION={}", sha1);
}
