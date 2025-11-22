fn main() {
    let rev = amimono_build::AppDigest::new()
        .add_glob("src/**/*.rs")
        .add_path("Cargo.toml")
        .compute();

    println!("cargo:rustc-env=APP_REVISION={}", rev);
}
