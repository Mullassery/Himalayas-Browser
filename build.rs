use std::env;

fn main() {
    // Generate build information
    println!("cargo:rustc-env=CARGO_CFG_TARGET_OS={}", env::var("CARGO_CFG_TARGET_OS").unwrap_or_default());
    println!("cargo:rustc-env=CARGO_CFG_TARGET_ARCH={}", env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default());

    // Platform detection
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    println!("cargo:rustc-env=PLATFORM={}_{}", target_os, target_arch);

    // Version information
    println!("cargo:rustc-env=BUILD_DATE={}", chrono::Local::now().to_rfc3339());
    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit_hash());

    // Platform-specific configuration
    match target_os.as_str() {
        "windows" => {
            println!("cargo:rustc-env=INSTALLER_TYPE=msi");
            println!("cargo:rustc-cfg=platform=\"windows\"");
        }
        "macos" => {
            println!("cargo:rustc-env=INSTALLER_TYPE=dmg");
            println!("cargo:rustc-cfg=platform=\"macos\"");
        }
        "linux" => {
            println!("cargo:rustc-env=INSTALLER_TYPE=deb");
            println!("cargo:rustc-cfg=platform=\"linux\"");
        }
        _ => {}
    }
}

fn git_commit_hash() -> String {
    std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
