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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
