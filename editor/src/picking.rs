use libdqg::camera::Camera;
use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::world::World;

/// Casts a ray from the camera through the mouse's normalized-device-coordinate position and
/// returns the nearest entity whose bounding sphere it intersects, if any.
///
/// Bounding-sphere picking (rather than exact mesh/triangle intersection) keeps this simple and
/// works uniformly for both `Sprite` and `Model` entities, which is all this milestone needs.
pub fn pick(world: &World, camera: &Camera, ndc_x: f32, ndc_y: f32) -> Option<Entity> {
    let inv_view_proj = camera.build_view_projection_matrix().inverse();

    let near = inv_view_proj.project_point3(glam::Vec3::new(ndc_x, ndc_y, 0.0));
    let far = inv_view_proj.project_point3(glam::Vec3::new(ndc_x, ndc_y, 1.0));
    let ray_origin = near;
    let ray_dir = (far - near).normalize();

    let mut best: Option<(Entity, f32)> = None;

    for (entity, renderable) in world.renderables.iter() {
        let Some(transform) = world.transforms.get(entity) else { continue };
        let matrix = transform.to_mat4();
        let center = matrix.transform_point3(renderable.bounding_center());
        // Radius scaled by the transform's largest axis scale, since bounding_radius() is in
        // local space but the sphere test needs to happen in world space.
        let scale = transform.scale.abs();
        let radius = renderable.bounding_radius() * scale.x.max(scale.y).max(scale.z);

        if let Some(t) = ray_sphere_intersection(ray_origin, ray_dir, center, radius) {
            if best.is_none_or(|(_, best_t)| t < best_t) {
                best = Some((entity, t));
            }
        }
    }

    best.map(|(entity, _)| entity)
}

fn ray_sphere_intersection(origin: glam::Vec3, dir: glam::Vec3, center: glam::Vec3, radius: f32) -> Option<f32> {
    let m = origin - center;
    let b = m.dot(dir);
    let c = m.dot(m) - radius * radius;
    if c > 0.0 && b > 0.0 {
        return None;
    }
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }
    let t = -b - discriminant.sqrt();
    Some(t.max(0.0))
}
