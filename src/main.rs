// Two native-only build-step modes, run before the wasm build (wasm has no
// linkme backend, so it can't collect translation catalogs itself, and its
// skrifa build panics on the raw MaterialIcons TTF — see src/i18n.rs's `web`
// module and src/theme.rs for the runtime-fetch side of both).
//
//   emf-mmf --gen-i18n <dir>              writes {bcp47}.cat per OFFERED language
//   emf-mmf --strip-icon-font <in> <out>  strips GPOS/GSUB so skrifa parses on wasm32
//
// This binary must be built natively (not --target wasm32-unknown-unknown) and
// with the default language features on, so linkme actually has catalogs to
// collect and `Strings`/`HarmonicStrings` are statically reached.

fn gen_i18n_dir() -> Option<std::path::PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--gen-i18n" {
            return args.next().map(std::path::PathBuf::from);
        }
    }
    None
}

fn generate_catalogs(dir: &std::path::Path) {
    std::fs::create_dir_all(dir).expect("create i18n output dir");
    for lang in emf_mmf::i18n::OFFERED {
        let bytes = i18n::export_catalogs(lang);
        let path = dir.join(format!("{}.cat", lang.bcp47()));
        std::fs::write(&path, &bytes).expect("write catalog bundle");
        eprintln!(
            "[gen-i18n] {} -> {} ({} bytes)",
            lang.bcp47(),
            path.display(),
            bytes.len()
        );
    }
}

fn strip_icon_font_paths() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--strip-icon-font" {
            let input = args.next().map(std::path::PathBuf::from)?;
            let output = args.next().map(std::path::PathBuf::from)?;
            return Some((input, output));
        }
    }
    None
}

/// skrifa's autohint_shaping reads GPOS/GSUB tables. The minimal stub tables in
/// MaterialIcons trigger ReadError::OffsetOutOfBounds on 32-bit wasm32. Rename
/// those tags to unknown values so skrifa silently skips them — glyph outlines
/// are unaffected, GPOS/GSUB only carry kerning/substitution rules.
fn strip_gpos_gsub(ttf: &[u8]) -> Vec<u8> {
    let mut out = ttf.to_vec();
    if out.len() < 12 {
        return out;
    }
    let num_tables = u16::from_be_bytes([out[4], out[5]]) as usize;
    for i in 0..num_tables {
        let base = 12 + i * 16;
        if base + 4 > out.len() {
            break;
        }
        let tag = &out[base..base + 4];
        if tag == b"GPOS" || tag == b"GSUB" {
            out[base] = b'X';
        }
    }
    out
}

fn strip_icon_font(input: &std::path::Path, output: &std::path::Path) {
    let raw = std::fs::read(input).expect("read input font");
    let stripped = strip_gpos_gsub(&raw);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).expect("create output dir");
    }
    std::fs::write(output, &stripped).expect("write stripped font");
    eprintln!(
        "[strip-icon-font] {} -> {} ({} bytes)",
        input.display(),
        output.display(),
        stripped.len()
    );
}

fn main() {
    if let Some(dir) = gen_i18n_dir() {
        generate_catalogs(&dir);
        return;
    }
    if let Some((input, output)) = strip_icon_font_paths() {
        strip_icon_font(&input, &output);
        return;
    }
    emf_mmf::main();
}
