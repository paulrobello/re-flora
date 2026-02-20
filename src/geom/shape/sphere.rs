use glam::Vec3;

use crate::geom::Aabb3;

#[derive(Debug, Clone)]
pub struct Sphere {
    center: Vec3,
    radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn center(&self) -> Vec3 {
        self.center
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn aabb(&self) -> Aabb3 {
        let r = Vec3::splat(self.radius);
        Aabb3::new(self.center - r, self.center + r)
    }
}
