pub mod app;
pub mod game;
pub mod renderer;
pub mod scene;
pub mod types;
pub mod input;
pub mod titlebar;
pub mod platform;
pub mod util;
pub mod camera;
pub mod transform;

pub use transform::Transformable;

/// Re-exported so downstream crates can build the vectors and matrices this API takes
/// (e.g. [`renderer::Sprite::transform`]) without pinning `glam` themselves.
pub use glam;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

/// Copies everything matching `res/*` in the calling crate into a `res/` folder
/// next to the built executable, so binaries can load assets through plain
/// relative paths (e.g. `./res/textures/smug.png`) no matter how they're started.
///
/// Meant to be called from a crate's `build.rs`; it relies on the `CARGO_MANIFEST_DIR`
/// and `OUT_DIR` variables Cargo sets for build scripts. Does nothing if the crate
/// has no `res/` directory.
pub fn copy_res_to_output_dir() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let res_dir = manifest_dir.join("res");
    if !res_dir.is_dir() {
        return Ok(());
    }
    println!("cargo:rerun-if-changed={}", res_dir.display());

    // `OUT_DIR` is `<target>/<profile>/build/<crate>-<hash>/out`, and the executable
    // is written three levels up from it, in `<target>/<profile>`.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let exe_dir = out_dir
        .ancestors()
        .nth(3)
        .ok_or("OUT_DIR is not laid out as <target>/<profile>/build/<crate>/out")?;
    let dest_dir = exe_dir.join("res");
    std::fs::create_dir_all(&dest_dir)?;

    let pattern = res_dir
        .join("*")
        .to_str()
        .ok_or("resource path is not valid UTF-8")?
        .replace('\\', "/");
    let entries = glob::glob(&pattern)?.collect::<Result<Vec<_>, _>>()?;

    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    fs_extra::copy_items(&entries, &dest_dir, &options)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
