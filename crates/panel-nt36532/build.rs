use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=PANEL_PROFILE");
    println!("cargo:rerun-if-changed=profile/generate_profile.py");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"))
        .join("display_profile.rs");
    let Some(profile) = env::var_os("PANEL_PROFILE") else {
        fs::write(
            output,
            "pub static SELECTED_PROFILE: Option<SelectedProfile> = None;\n",
        )
        .expect("write the default profile selection");
        return;
    };
    let profile = PathBuf::from(profile);
    println!("cargo:rerun-if-changed={}", profile.display());
    let status = Command::new("python3")
        .arg("profile/generate_profile.py")
        .arg(profile)
        .arg(output)
        .status()
        .expect("launch profile generator with Python 3.11 or later");
    assert!(status.success(), "panel profile generation failed");
}
