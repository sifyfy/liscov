fn main() {
    tauri_build::build();

    // テストバイナリに comctl32 v6 マニフェストを埋め込む。
    // tauri_build::build() はメインバイナリにのみマニフェストを埋め込むため、
    // 統合テストバイナリでは TaskDialogIndirect (comctl32 v6) が見つからず
    // STATUS_ENTRYPOINT_NOT_FOUND で起動に失敗する。
    #[cfg(windows)]
    embed_test_manifest();
}

#[cfg(windows)]
fn embed_test_manifest() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let manifest_path = std::path::PathBuf::from(&out_dir).join("test_comctl32v6.manifest");

    std::fs::write(
        &manifest_path,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>
"#,
    )
    .expect("マニフェストファイルの書き込みに失敗");

    let manifest_str = manifest_path.to_str().expect("OUT_DIR パスが非UTF-8");
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest_str
    );
}
