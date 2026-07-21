use glam::{Mat4, Quat, Vec3};

/// Basic affine transformations for anything that carries a model matrix.
///
/// Implementors only supply [`Transformable::transform`] and [`Transformable::transform_mut`]
/// (plus, optionally, a [`Transformable::pivot`]); every operation below is provided.
///
/// # Composition order
///
/// Each operation is post-multiplied onto the existing matrix, so it is applied in the object's
/// *local* space — the frame established by the transforms already on it. Chaining therefore
/// reads outside-in:
///
/// ```
/// # use libdqg::transform::Transformable;
/// # use libdqg::glam::{Mat4, Vec3};
/// # struct Thing(Mat4);
/// # impl Transformable for Thing {
/// #     fn transform(&self) -> Mat4 { self.0 }
/// #     fn transform_mut(&mut self) -> &mut Mat4 { &mut self.0 }
/// # }
/// # let mut thing = Thing(Mat4::IDENTITY);
/// // Rotate a quarter turn, then move 10 units along the *rotated* X axis.
/// thing.rotate(std::f32::consts::FRAC_PI_2).translate(10.0, 0.0);
/// ```
///
/// Rotations and scales pivot around [`Transformable::pivot`], which defaults to the local
/// origin. Translations are unaffected by the pivot. Use [`Transformable::set_transform`] when
/// you want to drive the matrix directly instead of accumulating onto it.
pub trait Transformable {
    /// The current model matrix.
    fn transform(&self) -> Mat4;

    /// Mutable access to the model matrix.
    fn transform_mut(&mut self) -> &mut Mat4;

    /// Local-space point that rotations and scales turn around. Defaults to the local origin.
    fn pivot(&self) -> Vec3 {
        Vec3::ZERO
    }

    /// Replaces the model matrix outright, discarding any accumulated transforms.
    fn set_transform(&mut self, transform: Mat4) -> &mut Self
    where
        Self: Sized,
    {
        *self.transform_mut() = transform;
        self
    }

    /// Clears the model matrix back to the identity.
    fn reset_transform(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self.set_transform(Mat4::IDENTITY)
    }

    /// Composes `transform` onto the current matrix, about the local origin.
    fn apply(&mut self, transform: Mat4) -> &mut Self
    where
        Self: Sized,
    {
        let current = self.transform();
        self.set_transform(current * transform)
    }

    /// Composes `transform` onto the current matrix, about [`Transformable::pivot`].
    fn apply_about_pivot(&mut self, transform: Mat4) -> &mut Self
    where
        Self: Sized,
    {
        let pivot = self.pivot();
        self.apply(Mat4::from_translation(pivot) * transform * Mat4::from_translation(-pivot))
    }

    /// Moves along the local X and Y axes.
    fn translate(&mut self, x: f32, y: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.translate3(Vec3::new(x, y, 0.0))
    }

    /// Moves by `offset` in local space.
    fn translate3(&mut self, offset: Vec3) -> &mut Self
    where
        Self: Sized,
    {
        self.apply(Mat4::from_translation(offset))
    }

    /// Rotates around the Z axis — the in-plane spin for 2D content.
    fn rotate(&mut self, radians: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.rotate_z(radians)
    }

    /// [`Transformable::rotate`], taking degrees.
    fn rotate_degrees(&mut self, degrees: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.rotate(degrees.to_radians())
    }

    /// Rotates around the X axis.
    fn rotate_x(&mut self, radians: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.apply_about_pivot(Mat4::from_rotation_x(radians))
    }

    /// Rotates around the Y axis.
    fn rotate_y(&mut self, radians: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.apply_about_pivot(Mat4::from_rotation_y(radians))
    }

    /// Rotates around the Z axis.
    fn rotate_z(&mut self, radians: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.apply_about_pivot(Mat4::from_rotation_z(radians))
    }

    /// Applies an arbitrary rotation.
    fn rotate_quat(&mut self, rotation: Quat) -> &mut Self
    where
        Self: Sized,
    {
        self.apply_about_pivot(Mat4::from_quat(rotation))
    }

    /// Scales the X and Y axes independently.
    fn scale(&mut self, x: f32, y: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.scale3(Vec3::new(x, y, 1.0))
    }

    /// Scales every axis by the same factor.
    fn scale_uniform(&mut self, factor: f32) -> &mut Self
    where
        Self: Sized,
    {
        self.scale3(Vec3::splat(factor))
    }

    /// Scales by `scale` in local space.
    fn scale3(&mut self, scale: Vec3) -> &mut Self
    where
        Self: Sized,
    {
        self.apply_about_pivot(Mat4::from_scale(scale))
    }

    /// Mirrors across the pivot's vertical axis.
    fn flip_x(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self.scale(-1.0, 1.0)
    }

    /// Mirrors across the pivot's horizontal axis.
    fn flip_y(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self.scale(1.0, -1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    const EPS: f32 = 1e-5;

    struct Thing {
        matrix: Mat4,
        pivot: Vec3,
    }

    impl Thing {
        fn new() -> Self {
            Self { matrix: Mat4::IDENTITY, pivot: Vec3::ZERO }
        }

        fn with_pivot(pivot: Vec3) -> Self {
            Self { matrix: Mat4::IDENTITY, pivot }
        }

        fn at(&self, point: Vec3) -> Vec3 {
            self.transform().transform_point3(point)
        }
    }

    impl Transformable for Thing {
        fn transform(&self) -> Mat4 {
            self.matrix
        }

        fn transform_mut(&mut self) -> &mut Mat4 {
            &mut self.matrix
        }

        fn pivot(&self) -> Vec3 {
            self.pivot
        }
    }

    #[test]
    fn translate_moves_the_origin() {
        let mut thing = Thing::new();
        thing.translate(3.0, -4.0);
        assert!(thing.at(Vec3::ZERO).abs_diff_eq(Vec3::new(3.0, -4.0, 0.0), EPS));
    }

    #[test]
    fn rotation_holds_the_pivot_fixed() {
        let pivot = Vec3::new(8.0, 5.0, 0.0);
        let mut thing = Thing::with_pivot(pivot);
        thing.rotate(FRAC_PI_2);

        assert!(thing.at(pivot).abs_diff_eq(pivot, EPS));
        // A point one unit right of the pivot swings to one unit above it.
        assert!(thing.at(pivot + Vec3::X).abs_diff_eq(pivot + Vec3::Y, EPS));
    }

    #[test]
    fn scale_expands_away_from_the_pivot() {
        let pivot = Vec3::new(2.0, 2.0, 0.0);
        let mut thing = Thing::with_pivot(pivot);
        thing.scale_uniform(3.0);

        assert!(thing.at(pivot).abs_diff_eq(pivot, EPS));
        assert!(thing.at(pivot + Vec3::new(1.0, 1.0, 0.0))
            .abs_diff_eq(pivot + Vec3::new(3.0, 3.0, 0.0), EPS));
    }

    #[test]
    fn flip_x_mirrors_across_the_pivot() {
        let pivot = Vec3::new(10.0, 0.0, 0.0);
        let mut thing = Thing::with_pivot(pivot);
        thing.flip_x();

        assert!(thing.at(Vec3::new(12.0, 4.0, 0.0)).abs_diff_eq(Vec3::new(8.0, 4.0, 0.0), EPS));
    }

    #[test]
    fn chained_operations_apply_in_local_space() {
        let mut thing = Thing::new();
        thing.rotate(FRAC_PI_2).translate(10.0, 0.0);

        // The translation runs along the rotated X axis, which now points up.
        assert!(thing.at(Vec3::ZERO).abs_diff_eq(Vec3::new(0.0, 10.0, 0.0), EPS));
    }

    #[test]
    fn set_transform_discards_accumulated_state() {
        let mut thing = Thing::new();
        thing.translate(5.0, 5.0).set_transform(Mat4::from_translation(Vec3::X));
        assert!(thing.at(Vec3::ZERO).abs_diff_eq(Vec3::X, EPS));

        thing.reset_transform();
        assert!(thing.at(Vec3::ZERO).abs_diff_eq(Vec3::ZERO, EPS));
    }
}
