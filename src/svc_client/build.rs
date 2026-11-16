use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let bundled_vendor = manifest_dir.join("vendor");
    let out_vendor = if bundled_vendor.is_dir() {
        bundled_vendor
    } else {
        restore_bundled_opus(&manifest_dir)
    };
    compile_opus(&out_vendor);
}

fn restore_bundled_opus(manifest_dir: &Path) -> PathBuf {
    let encoded = std::fs::read_to_string(manifest_dir.join("opus_sources.rs"))
        .expect("native mod package is missing embedded Opus sources");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("opus-vendor");
    let mut current: Option<PathBuf> = None;
    for line in encoded.lines() {
        if let Some(name) = line.strip_prefix("// FILE: ") {
            let relative = Path::new(name);
            assert!(!relative.is_absolute() && !relative.components().any(|c| matches!(c, std::path::Component::ParentDir)), "invalid embedded Opus path");
            current = Some(out.join(relative));
        } else if let Some(hex) = line.strip_prefix("// HEX: ") {
            let path = current.take().expect("Opus HEX entry without FILE entry");
            let bytes = decode_hex(hex);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
    }
    out
}

fn decode_hex(input: &str) -> Vec<u8> {
    assert!(input.len() % 2 == 0, "invalid embedded Opus hex");
    input.as_bytes().chunks_exact(2).map(|pair| {
        let digit = |b: u8| match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => panic!("invalid embedded Opus hex"),
        };
        digit(pair[0]) * 16 + digit(pair[1])
    }).collect()
}

fn compile_opus(root: &Path) {
    let mut build = cc::Build::new();
    build
        .define("HAVE_CONFIG_H", None)
        .include(root)
        .include(root.join("include"))
        .include(root.join("celt"))
        .include(root.join("silk"))
        .include(root.join("silk/float"))
        .include(root.join("src"))
        .flag_if_supported("-O2");
    let mut n = 0;
    for dir in ["celt", "silk", "silk/float", "src"] {
        for entry in walk(&root.join(dir)) {
            let ps = entry.to_string_lossy().replace('\\', "/");
            let name = entry.file_name().unwrap().to_string_lossy().to_string();
            let skip = ps.contains("test")
                || ps.contains("demo")
                || ps.contains("compare")
                || ps.contains("/arm/")
                || ps.contains("/x86/")
                || ps.contains("/fixed/");
            if name.ends_with(".c") && !skip {
                build.file(entry);
                n += 1;
            }
        }
    }
    println!("cargo:warning=svc_client: compiling {} libopus C sources", n);
    build.compile("opus");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
