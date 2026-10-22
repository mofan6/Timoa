fn check_ui_font_coverage() {
    use std::{collections::BTreeSet, path::Path};
    fn scan(path: &Path, chars: &mut BTreeSet<char>) {
        if path.is_dir() {
            for entry in std::fs::read_dir(path).unwrap().flatten() { scan(&entry.path(), chars); }
        } else if matches!(path.extension().and_then(|part| part.to_str()), Some("rs" | "slint" | "json")) {
            println!("cargo:rerun-if-changed={}", path.display());
            chars.extend(std::fs::read_to_string(path).unwrap().chars().filter(|ch| matches!(*ch as u32,
                0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af)));
        }
    }
    let coverage = Path::new("assets/fonts/coverage.txt");
    println!("cargo:rerun-if-changed={}", coverage.display());
    println!("cargo:rerun-if-changed=tools/font-common-characters.txt");
    println!("cargo:rerun-if-changed=tools/subset-ui-fonts.py");
    println!("cargo:rerun-if-env-changed=TIMO_FONT_PYTHON");
    println!("cargo:rerun-if-env-changed=TIMO_FONT_MASTER_DIR");
    let mut required = BTreeSet::new();
    for directory in ["src", "ui", "i18n"] { scan(Path::new(directory), &mut required); }
    let bundled: BTreeSet<char> = std::fs::read_to_string(coverage).unwrap_or_default().chars().collect();
    if required.is_subset(&bundled) { return; }
    let python = std::env::var_os("TIMO_FONT_PYTHON");
    let master = std::env::var_os("TIMO_FONT_MASTER_DIR");
    if let (Some(python), Some(master)) = (python, master) {
        let status = std::process::Command::new(python).arg("tools/subset-ui-fonts.py")
            .arg("--master-dir").arg(master).status().expect("Font subset generator");
        assert!(status.success(), "Font subset generation failed");
        let regenerated: BTreeSet<char> = std::fs::read_to_string(coverage).unwrap().chars().collect();
        assert!(required.is_subset(&regenerated), "New UI glyphs were not included");
    } else {
        panic!("New Chinese UI text needs font glyphs. Run tools/subset-ui-fonts.py with the full upstream masters, or set TIMO_FONT_PYTHON and TIMO_FONT_MASTER_DIR for automatic generation.");
    }
}

fn main() {
    check_ui_font_coverage();
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
        // Embed images and common-character subsets of Sarasa UI SC fonts.
        // Keep vector outlines available for dynamic text and native rasterization.
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles)).expect("Slint UI compilation");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            // Rust's bundled MinGW CRT omits this Windows shell import library.
            // Copy only the import library so an unrelated compiler CRT is never linked.
            let compiler = if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86") { "i686-w64-mingw32-gcc" } else { "x86_64-w64-mingw32-gcc" };
            if let Ok(output) = std::process::Command::new(compiler).arg("-print-file-name=libshlwapi.a").output() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if std::path::Path::new(&path).is_file() {
                    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
                    std::fs::copy(path, out.join("libshlwapi.a")).expect("Windows shell import library");
                    println!("cargo:rustc-link-search=native={}", out.display());
                }
            }
        }
        let mut resource = winresource::WindowsResource::new();
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            resource.set_windres_path(if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86") { "i686-w64-mingw32-windres" } else { "x86_64-w64-mingw32-windres" });
        }
        resource.set_icon("assets/timoa.ico");
        resource.set("ProductName", "Timoa");
        resource.set("FileDescription", "Timoa 原生声明式计时与闹钟");
        resource.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        resource.set_manifest_file("app.manifest");
        resource.compile().expect("Windows application resources");
    }
}
