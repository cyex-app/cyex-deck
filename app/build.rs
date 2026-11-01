fn main() {
    // V108-N26 The Discord app ID (public, but kept out of the code like a secret): the
    // environment's DISCORD_CLIENT_ID, else the same line in the repo's .env. Missing or not
    // all digits: nothing is set and Rich Presence stays off in this build.
    println!("cargo:rerun-if-env-changed=DISCORD_CLIENT_ID");
    println!("cargo:rerun-if-changed=../.env");
    if let Some(id) = discord_client_id() {
        println!("cargo:rustc-env=DISCORD_CLIENT_ID={id}");
    }

    // V107-R2 Tauri's manifest (common controls) plus the segment heap.
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows-app-manifest.xml"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri build script failed");
}

fn discord_client_id() -> Option<String> {
    let from_env = std::env::var("DISCORD_CLIENT_ID").ok();
    let from_file = || {
        let text = std::fs::read_to_string("../.env").ok()?;
        text.lines().find_map(|l| l.trim().strip_prefix("DISCORD_CLIENT_ID=").map(|v| v.trim().trim_matches('"').to_owned()))
    };
    from_env
        .filter(|v| !v.trim().is_empty())
        .or_else(from_file)
        .map(|v| v.trim().to_owned())
        .filter(|v| (15..=21).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_digit()))
}
