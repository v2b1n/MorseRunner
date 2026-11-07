use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=MORSE_RUNNER_BUILD_VERSION");

    let version = env::var("MORSE_RUNNER_BUILD_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned());
    if !is_numeric_semver(&version) {
        panic!("MORSE_RUNNER_BUILD_VERSION must be MAJOR.MINOR.PATCH, got {version:?}");
    }

    println!("cargo:rustc-env=MORSE_RUNNER_BUILD_VERSION={version}");
}

fn is_numeric_semver(version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && (*part == "0" || !part.starts_with('0'))
                && part.bytes().all(|byte| byte.is_ascii_digit())
        })
}
