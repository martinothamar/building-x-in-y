#![feature(stmt_expr_attributes)]
#![feature(const_trait_impl)]
#![feature(const_fn_floating_point_arithmetic)]
#![allow(unused)]

use std::{fmt::Display, io::Write};

use ray::Ray;
use vec3::Point3;

use crate::{color::Color, vec3::Vec3};

mod color;
mod hittable;
mod ray;
mod vec3;

fn hit_sphere(center: &Point3, radius: f64, r: &Ray) -> f64 {
    let oc = r.origin() - center;
    let a = r.direction().len_squared();
    let half_b = Vec3::dot(&oc, r.direction());
    let c = oc.len_squared() - radius * radius;
    let discriminant = half_b * half_b - a * c;
    if discriminant < 0.0 {
        -1.0
    } else {
        (-half_b - discriminant.sqrt()) / a
    }
}

fn ray_color(r: &Ray) -> Color {
    let t = hit_sphere(&vec3![0.0, 0.0, -1.0], 0.5, r);
    if t > 0.0 {
        let n = (r.at(t) - vec3![0.0, 0.0, -1.0]).unit_vector();
        return color![n.x() + 1.0, n.y() + 1.0, n.z() + 1.0] * 0.5;
    }

    let unit_direction = r.direction().unit_vector();
    let a = (unit_direction.y() + 1.0) * 0.5;

    (color![1.0, 1.0, 1.0] * (1.0 - a)) + (color![0.5, 0.7, 1.0] * a)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aspect_ratio = 16.0 / 9.0;
    let image_width = 400;
    let image_height = (image_width as f64 / aspect_ratio) as i32;
    let image_height = image_height.max(1);

    let focal_length = 1.0;
    let viewport_height = 2.0;
    let viewport_width = viewport_height * (image_width as f64 / image_height as f64);
    let camer_center = Point3::new();

    let viewport_u = vec3![viewport_width, 0.0, 0.0];
    let viewport_v = vec3![0.0, -viewport_height, 0.0];

    let pixel_delta_u = viewport_u / image_width as f64;
    let pixel_detla_v = viewport_v / image_height as f64;

    let viewport_upper_left = camer_center - vec3![0.0, 0.0, focal_length] - viewport_u / 2.0 - viewport_v / 2.0;
    let pixel00_loc = viewport_upper_left + (pixel_delta_u + pixel_detla_v) * 0.5;

    let mut stdout = std::io::stdout().lock();

    write!(stdout, "P3\n{image_width} {image_height}\n255\n")?;

    for j in 0..image_height {
        for i in 0..image_width {
            // auto pixel_center = pixel00_loc + (i * pixel_delta_u) + (j * pixel_delta_v);
            // auto ray_direction = pixel_center - camera_center;
            // ray r(camera_center, ray_direction);

            // color pixel_color = ray_color(r);

            let pixel_center = pixel00_loc + (pixel_delta_u * i as f64) + (pixel_detla_v * j as f64);
            let ray_direction = pixel_center - camer_center;
            let r = Ray::new(camer_center, ray_direction);
            let pixel_color = ray_color(&r);
            write!(stdout, "{}", pixel_color.as_ppm())?;
        }
    }

    Ok(())
}

fn render_simple_ppm() -> Result<(), Box<dyn std::error::Error>> {
    let image_width = 256;
    let image_height = 256;

    let mut stdout = std::io::stdout().lock();

    write!(stdout, "P3\n{image_width} {image_height}\n255\n")?;

    for j in 0..image_height {
        for i in 0..image_width {
            let color = Color::new_with(
                i as f64 / (image_width - 1) as f64,
                j as f64 / (image_height - 1) as f64,
                0.0,
            );
            write!(stdout, "{color}")?;
        }
    }

    Ok(())
}
