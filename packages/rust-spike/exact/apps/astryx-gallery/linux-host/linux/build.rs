//! Compile and bake the gallery's `app.contract` into `OUT_DIR/app.plan`, with the entry the
//! Linux host runs. Follows Exact's Rust-data apps; the gallery has no widgets and no hatches.

use contract::DataSource;

fn main() {
    let app_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    contract::rerun_if_changed(&app_dir.join("app.contract"));
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", app_dir.join("app.json").display());
    let plan = contract::compile_path(&app_dir.join("app.contract")).unwrap_or_else(|e| panic!("app.contract:{e}"));
    let entries = contract::snapshot::entries(&app_dir).unwrap();
    let baked = contract::bake(plan, astryx_gallery_data::Gallery).unwrap_or_else(|e| panic!("bake: {e:?}"));
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("app.plan"), baked.encode()).unwrap();
    let target = std::env::var("TARGET").unwrap_or_default();
    let manifest = contract::Manifest::read(&app_dir).unwrap_or_else(|e| panic!("app.json: {e}"));
    let grants = astryx_gallery_data::Gallery.grants();
    let compat = exact_bake::compatibility_id_sources_with_entries(&app_dir, "linux", &target, &manifest, Some(grants), None, Some(&entries))
        .unwrap_or_else(|e| panic!("compatibility id: {e}"));
    std::fs::write(out_dir.join("compat.json"), compat.to_json()).unwrap();
    let (hatches, run) = contract::native::rust_hatch_entry(&app_dir, &manifest, &target).unwrap_or_else(|e| panic!("hatches: {e}"));
    std::fs::write(
        out_dir.join("entry.rs"),
        contract::linux_entry(
            "astryx_gallery_data::Gallery",
            "astryx_gallery_data::Gallery",
            compat.inputs["rustMode"].as_str().unwrap(),
            "exact_linux",
            &hatches,
            run,
        )
        .unwrap_or_else(|e| panic!("entry: {e}")),
    )
    .unwrap();
}
