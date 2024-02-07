use std::{env, io};
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};

use flate2::Compression;

mod files;

fn main() {
    let workspace_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..").canonicalize().unwrap();
    let webapp_root = workspace_dir.join("webapp/dist");
    if env::var("PROFILE").unwrap() == "release" {
        embed_files(webapp_root);
    } else {
        fetch_local_files(webapp_root);
    }
}

fn embed_files(root: PathBuf) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_files.rs");
    let mut output_file = BufWriter::new(File::create(&dest_path).unwrap());

    let mut map = phf_codegen::Map::new();
    let mut file_counter = 0;
    for path in files::collect_files(&root).unwrap() {
        let input_file = File::open(&path).unwrap();
        let mut reader = flate2::read::GzEncoder::new(input_file, Compression::default());
        let filename = format!("{file_counter:>06}-{}.gz", path.file_name().unwrap().to_string_lossy());
        let mut writer = File::create(Path::new(&out_dir).join(&filename)).unwrap();
        io::copy(&mut reader, &mut writer).unwrap();
        let include_bytes = format!("include_bytes!(concat!(env!(\"OUT_DIR\"), '{}', {filename:?}))", std::path::MAIN_SEPARATOR);
        let path_key = format!("{}", path.strip_prefix(&root).unwrap().display());
        map.entry(path_key, &format!("EmbeddedFile {{\
            data: {include_bytes},\
            mime_type: {:?},\
            compressed: true,\
        }}", mime_guess::from_path(&path).first_or_text_plain().essence_str()));
        file_counter += 1;
    }

    write!(
        &mut output_file,
        "static EMBEDDED_FILES: phf::Map<&'static str, EmbeddedFile> = {};\n",
        map.build()
    ).unwrap();
}

fn fetch_local_files(root: PathBuf) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("embedded_files.rs");
    let mut output_file = BufWriter::new(File::create(&dest_path).unwrap());
    write!(&mut output_file, "static WEBAPP_ROOT: &'static str = {:?};\n", root.to_str().unwrap()).unwrap();
}