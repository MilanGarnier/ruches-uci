fn main() {
    let profile = std::env::var("PROFILE").unwrap();
    println!("cargo:rustc-env=LOG_LEVEL={}", match profile.as_str() {
        "release" => "ERROR",
        "dev" => "DEBUG",
        "fixme" => "TRACE",
        _ => "INFO",
    });
}
