fn main() {
    // V107-R2 Tauri's manifest (common controls) plus the segment heap.
    let windows = tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows-app-manifest.xml"));
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri build script failed");
}
