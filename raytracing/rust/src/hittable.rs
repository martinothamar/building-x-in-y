use crate::{
    ray::Ray,
    vec3::{Point3, Vec3},
};

pub struct HitRecord {
    p: Point3,
    normal: Vec3,
    t: f64,
}

pub trait Hittable {
    fn hit(&self, r: &Ray, ray_tmin: f64, ray_tmax: f64, rec: &mut HitRecord) -> bool;
}

pub struct Sphere {
    center: Point3,
    radius: f64,
}

impl Hittable for Sphere {
    fn hit(&self, r: &Ray, ray_tmin: f64, ray_tmax: f64, rec: &mut HitRecord) -> bool {
        let oc = r.origin() - &self.center;
        let a = r.direction().len_squared();
        let half_b = Vec3::dot(&oc, r.direction());
        let c = oc.len_squared() - self.radius * self.radius;

        let discriminant = half_b * half_b - a * c;
        if discriminant < 0.0 {
            return false;
        }

        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd) / a;
        if root <= ray_tmin || ray_tmax <= root {
            root = (-half_b + sqrtd) / a;
            if root <= ray_tmin || ray_tmax <= root {
                return false;
            }
        }

        rec.t = root;
        rec.p = r.at(rec.t);
        rec.normal = (rec.p - self.center) / self.radius;

        true
    }
}
