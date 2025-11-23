fn main() {
    let rev = amimono_build::AppDigest::new()
        .add_glob("../amimono/src/**/*.rs")
        .add_path("../amimono/Cargo.toml")
        .add_glob("../amimono-build/src/**/*.rs")
        .add_path("../amimono-build/Cargo.toml")
        .add_glob("src/**/*.rs")
        .add_path("Cargo.toml")
        .compute();

    println!("cargo:rustc-env=APP_REVISION={}", rev);
}
