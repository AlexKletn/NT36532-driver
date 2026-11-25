use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=TOUCH_PROFILES_DIR");
    println!("cargo:rerun-if-changed=profile/generate_profile.py");

    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR")).join("profile.rs");
    let Some(directory) = env::var_os("TOUCH_PROFILES_DIR") else {
        fs::write(
            output,
            "pub static TOUCH_PROFILES: &[FirmwareProfile] = &[];\n",
        )
        .expect("write the empty touch profile table");
        return;
    };
    let directory = PathBuf::from(directory);
    println!("cargo:rerun-if-changed={}", directory.display());
    let status = Command::new("python3")
        .arg("profile/generate_profile.py")
        .arg(directory)
        .arg(output)
        .status()
        .expect("launch profile generator with Python 3.11 or later");
    assert!(status.success(), "touch profile generation failed");
}
