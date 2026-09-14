use libdqg::camera::Camera;
use libdqg::ecs::Entity;
use libdqg::glam;
use libdqg::renderer::Model;
use libdqg::world::{Renderable, World};

/// Casts a ray from the camera through the mouse's normalized-device-coordinate position and
/// returns the nearest entity it intersects, if any.
///
/// `Model` entities are tested against their actual mesh triangles (ray/triangle intersection
/// against each mesh's local-space geometry, transformed into world space), so picking follows
/// the model's real silhouette rather than a loose bounding volume. `Sprite` entities still use a
/// bounding-sphere test, since a sprite is already just a flat quad.
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

        let hit_t = match renderable {
            Renderable::Model(model) => pick_model(model, matrix, ray_origin, ray_dir),
            Renderable::Sprite(_) => {
                let center = matrix.transform_point3(renderable.bounding_center());
                // Radius scaled by the transform's largest axis scale, since bounding_radius() is
                // in local space but the sphere test needs to happen in world space.
                let scale = transform.scale.abs();
                let radius = renderable.bounding_radius() * scale.x.max(scale.y).max(scale.z);
                ray_sphere_intersection(ray_origin, ray_dir, center, radius)
            }
        };

        if let Some(t) = hit_t {
            if best.is_none_or(|(_, best_t)| t < best_t) {
                best = Some((entity, t));
            }
        }
    }

    best.map(|(entity, _)| entity)
}

/// Nearest ray/triangle hit (by `t`) across every triangle of every mesh in `model`, with local
/// mesh positions transformed into world space by `matrix` first.
fn pick_model(model: &Model, matrix: glam::Mat4, ray_origin: glam::Vec3, ray_dir: glam::Vec3) -> Option<f32> {
    let mut nearest: Option<f32> = None;

    for mesh in &model.meshes {
        for tri in mesh.indices.chunks_exact(3) {
            let v0 = matrix.transform_point3(mesh.positions[tri[0] as usize]);
            let v1 = matrix.transform_point3(mesh.positions[tri[1] as usize]);
            let v2 = matrix.transform_point3(mesh.positions[tri[2] as usize]);

            if let Some(t) = ray_triangle_intersection(ray_origin, ray_dir, v0, v1, v2) {
                if nearest.is_none_or(|n| t < n) {
                    nearest = Some(t);
                }
            }
        }
    }

    nearest
}

/// Möller–Trumbore ray/triangle intersection. Double-sided (doesn't consider winding order),
/// since picking should hit a triangle regardless of which way it faces the camera.
fn ray_triangle_intersection(
    origin: glam::Vec3, dir: glam::Vec3, v0: glam::Vec3, v1: glam::Vec3, v2: glam::Vec3,
) -> Option<f32> {
    const EPSILON: f32 = 1e-6;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let h = dir.cross(edge2);
    let a = edge1.dot(h);
    if a.abs() < EPSILON {
        return None; // Ray is parallel to the triangle.
    }

    let f = 1.0 / a;
    let s = origin - v0;
    let u = f * s.dot(h);
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);
    if t > EPSILON { Some(t) } else { None }
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
