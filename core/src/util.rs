use std::path::{Path, PathBuf};

/// Resolves a resource path for the running binary.
///
/// Absolute paths are returned untouched. A relative path is first looked for next to
/// the executable — where [`crate::copy_res_to_output_dir`] puts `res/` at build time —
/// so a binary finds its assets no matter what the working directory is. If nothing is
/// there, the path is returned unchanged and stays relative to the working directory,
/// which is what `cargo run` from a package root relies on.
pub fn resolve_resource_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        return path.to_path_buf();
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let candidate = exe_dir.join(path);
        if candidate.exists() {
            return candidate;
        }
    }

    path.to_path_buf()
}

pub fn slice_to_bytes<T>(slice: &[T]) -> &[u8] {
    let size = std::mem::size_of::<T>();
    let len = slice.len() * size;
    let ptr = slice.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}