use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=schema_manifest.json");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let src = PathBuf::from("schema_manifest.json");
    let dst = out_dir.join("schema_manifest.json");
    let payload = fs::read_to_string(&src).expect("read schema_manifest.json");
    fs::write(&dst, payload).expect("write schema manifest");
}
