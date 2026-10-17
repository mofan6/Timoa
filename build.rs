fn main() {
    let catalog: std::collections::BTreeMap<String, [String; 2]> = serde_json::from_str(
        &std::fs::read_to_string("i18n/messages.json").expect("Translation catalog")
    ).expect("Valid translation catalog");
    let bundle = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("translations");
    for (language, index) in [("zh_CN", None), ("zh_TW", Some(0)), ("en", Some(1))] {
        let directory = bundle.join(language).join("LC_MESSAGES");
        std::fs::create_dir_all(&directory).unwrap();
        let mut po = String::from("msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n");
        for (source, translated) in &catalog {
            let target = index.map(|i| translated[i].as_str()).unwrap_or(source);
            po += &format!("msgid {}\nmsgstr {}\n\n", serde_json::to_string(source).unwrap(), serde_json::to_string(target).unwrap());
        }
        std::fs::write(directory.join("timoa.po"), po).unwrap();
    }
    println!("cargo:rerun-if-changed=i18n/messages.json");
    println!("cargo:rerun-if-changed=app.manifest");
    slint_build::compile_with_config("ui/app.slint", slint_build::CompilerConfiguration::new()
        .with_style("fluent-dark".into())
        .with_default_translation_context(slint_build::DefaultTranslationContext::None)
        .with_bundled_translations(bundle)
        // Keep image files compressed and resolve Windows vector fonts at runtime.
        // Pre-rasterized embedded fonts greatly inflate a desktop binary and omit dynamic glyphs.
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles)).expect("Slint UI compilation");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            // Rust's bundled MinGW CRT omits this Windows shell import library.
            // Copy only the import library so an unrelated compiler CRT is never linked.
            if let Ok(output) = std::process::Command::new("gcc").arg("-print-file-name=libshlwapi.a").output() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if std::path::Path::new(&path).is_file() {
                    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
                    std::fs::copy(path, out.join("libshlwapi.a")).expect("Windows shell import library");
                    println!("cargo:rustc-link-search=native={}", out.display());
                }
            }
        }
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/timoa.ico");
        resource.set("ProductName", "Timoa");
        resource.set("FileDescription", "Timoa 原生声明式计时与闹钟");
        resource.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        resource.set_manifest_file("app.manifest");
        resource.compile().expect("Windows application resources");
    }
}
