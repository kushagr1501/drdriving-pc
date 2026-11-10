//! Wavefront OBJ models as the original game loads them: positions + texture coordinates only,
//! faces taken as triangles, `v` flipped (`1 - vt.y`), no lighting.

use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;
use std::cell::RefCell;

pub struct Model {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    wheels: Vec<Wheel>,
    cache: RefCell<Option<CachedGeometry>>,
}

struct Wheel {
    pos: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    pivot: Vec3,
    front: bool,
    cache: RefCell<Option<CachedGeometry>>,
}

struct CachedGeometry {
    chunks: Vec<Mesh>,
    color: [u8; 4],
    matrix: Option<Mat4>,
}

fn wheel_transform(pivot: Vec3, spin: f32, steering: f32) -> Mat4 {
    Mat4::from_translation(pivot) * Mat4::from_rotation_y(steering)
        * Mat4::from_rotation_x(spin) * Mat4::from_translation(-pivot)
}

fn index(tok: &str, count: usize) -> usize {
    let i: i32 = tok.parse().unwrap_or(1);
    if i < 0 {
        (count as i32 + i) as usize
    } else {
        (i - 1) as usize
    }
}

impl Model {
    /// Strict import path: triangles, finite positions and valid UV references are required.
    /// Fits the model uniformly into the selected car's stock collision footprint.
    pub fn import_obj(text: &str, half_width: f32, half_length: f32, height: f32) -> Result<Model, String> {
        let mut vertices = Vec::<[f32; 3]>::new();
        let mut texcoords = Vec::<[f32; 2]>::new();
        let mut pos = Vec::new();
        let mut uv = Vec::new();
        let mut face_wheels = Vec::new();
        let mut current_wheel = None;
        let resolve = |value: &str, count: usize| -> Result<usize, String> {
            let n = value.parse::<i64>().map_err(|_| "Invalid OBJ index.")?;
            let i = if n > 0 { n - 1 } else if n < 0 { count as i64 + n } else { -1 };
            if i < 0 || i >= count as i64 { return Err("OBJ index is out of range.".into()); }
            Ok(i as usize)
        };
        for line in text.lines() {
            let mut words = line.split('#').next().unwrap_or("").split_whitespace();
            match words.next() {
                Some(kind @ ("v" | "vt")) => {
                    let is_uv = kind == "vt";
                    let required = if is_uv { 2 } else { 3 };
                    let values: Vec<f32> = words.take(required).map(|w| w.parse::<f32>())
                        .collect::<Result<_, _>>().map_err(|_| "Invalid OBJ coordinate.")?;
                    if values.len() != required || values.iter().any(|v| !v.is_finite() || v.abs() > 1.0e8) {
                        return Err("OBJ coordinates must be finite and within range.".into());
                    }
                    if is_uv { texcoords.push([values[0], 1.0 - values[1]]); }
                    else { vertices.push([values[0], values[1], values[2]]); }
                }
                Some("f") => {
                    let face: Vec<_> = words.collect();
                    if face.len() != 3 { return Err("Export the OBJ with Triangulate Faces enabled.".into()); }
                    if pos.len() >= 60_000 { return Err("Use a model with at most 20,000 triangles.".into()); }
                    face_wheels.push(current_wheel);
                    for corner in face {
                        let mut parts = corner.split('/');
                        let v = resolve(parts.next().unwrap_or(""), vertices.len())?;
                        let t = resolve(parts.next().unwrap_or(""), texcoords.len())?;
                        pos.push(vertices[v]);
                        uv.push(texcoords[t]);
                    }
                }
                Some("o") => {
                    current_wheel = match words.next() {
                        Some("wheel_fl") => Some(0), Some("wheel_fr") => Some(1),
                        Some("wheel_rl") => Some(2), Some("wheel_rr") => Some(3), _ => None,
                    };
                }
                _ => {}
            }
        }
        if pos.is_empty() { return Err("The OBJ contains no triangles.".into()); }
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &pos { for i in 0..3 { lo[i] = lo[i].min(p[i]); hi[i] = hi[i].max(p[i]); } }
        let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        if size.iter().any(|s| *s < 1.0e-6) { return Err("The model must have width, height and length.".into()); }
        let scale = (half_width * 2.0 / size[0]).min(height / size[1]).min(half_length * 2.0 / size[2]);
        for p in &mut pos {
            p[0] = (p[0] - (lo[0] + hi[0]) * 0.5) * scale;
            p[1] = (p[1] - lo[1]) * scale;
            p[2] = (p[2] - (lo[2] + hi[2]) * 0.5) * scale;
        }
        let mut model = Model { pos, uv, wheels: Vec::new(), cache: RefCell::new(None) };
        model.partition_wheels(&face_wheels)?;
        Ok(model)
    }

    fn partition_wheels(&mut self, groups: &[Option<usize>]) -> Result<(), String> {
        if groups.iter().all(Option::is_none) { return Ok(()); }
        let mut body_pos = Vec::new();
        let mut body_uv = Vec::new();
        let mut wheel_pos: [Vec<[f32; 3]>; 4] = std::array::from_fn(|_| Vec::new());
        let mut wheel_uv: [Vec<[f32; 2]>; 4] = std::array::from_fn(|_| Vec::new());
        for (face, group) in groups.iter().enumerate() {
            let range = face * 3..face * 3 + 3;
            match group {
                Some(w) => { wheel_pos[*w].extend_from_slice(&self.pos[range.clone()]); wheel_uv[*w].extend_from_slice(&self.uv[range]); }
                None => { body_pos.extend_from_slice(&self.pos[range.clone()]); body_uv.extend_from_slice(&self.uv[range]); }
            }
        }
        if body_pos.is_empty() || wheel_pos.iter().any(|p| p.len() < 6) {
            return Err("A wheel rig needs a body and all four wheel_fl/fr/rl/rr objects.".into());
        }
        let mut wheels = Vec::new();
        for (i, (pos, uv)) in wheel_pos.into_iter().zip(wheel_uv).enumerate() {
            let mut lo = Vec3::splat(f32::MAX);
            let mut hi = Vec3::splat(f32::MIN);
            for p in &pos { let p = Vec3::from_array(*p); lo = lo.min(p); hi = hi.max(p); }
            wheels.push(Wheel { pos, uv, pivot: (lo + hi) * 0.5, front: i < 2, cache: RefCell::new(None) });
        }
        self.pos = body_pos;
        self.uv = body_uv;
        self.wheels = wheels;
        Ok(())
    }

    pub fn wheel_count(&self) -> usize { self.wheels.len() }

    pub fn parse(text: &str) -> Model {
        let mut v: Vec<[f32; 3]> = Vec::new();
        let mut vt: Vec<[f32; 2]> = Vec::new();
        let mut pos = Vec::new();
        let mut uv = Vec::new();
        for line in text.lines() {
            let mut it = line.split_whitespace();
            match it.next() {
                Some("v") => {
                    let p: Vec<f32> = it.filter_map(|s| s.parse().ok()).collect();
                    if p.len() >= 3 {
                        v.push([p[0], p[1], p[2]]);
                    }
                }
                Some("vt") => {
                    let p: Vec<f32> = it.filter_map(|s| s.parse().ok()).collect();
                    if p.len() >= 2 {
                        vt.push([p[0], 1.0 - p[1]]);
                    }
                }
                Some("f") => {
                    for corner in it.take(3) {
                        let mut parts = corner.split('/');
                        let vi = index(parts.next().unwrap_or("1"), v.len());
                        pos.push(v.get(vi).copied().unwrap_or([0.0; 3]));
                        let t = parts.next().filter(|s| !s.is_empty()).map(|s| index(s, vt.len()));
                        uv.push(t.and_then(|i| vt.get(i).copied()).unwrap_or([0.0, 0.0]));
                    }
                }
                _ => {}
            }
        }
        Model { pos, uv, wheels: Vec::new(), cache: RefCell::new(None) }
    }

    /// UV bounding box (u0, v0, u1, v1, in 0..1 image space, top-down) of the flat shadow quad baked
    /// into car models: the triangles lying on the model's lowest plane.
    pub fn shadow_uv_rect(&self) -> Option<[f32; 4]> {
        let min_y = self.pos.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let mut r: Option<[f32; 4]> = None;
        for t in 0..self.pos.len() / 3 {
            if (0..3).all(|k| (self.pos[t * 3 + k][1] - min_y).abs() < 1e-3) {
                for k in 0..3 {
                    let uv = self.uv[t * 3 + k];
                    let b = r.get_or_insert([uv[0], uv[1], uv[0], uv[1]]);
                    b[0] = b[0].min(uv[0]);
                    b[1] = b[1].min(uv[1]);
                    b[2] = b[2].max(uv[0]);
                    b[3] = b[3].max(uv[1]);
                }
            }
        }
        r
    }

    pub fn load(path: &std::path::Path) -> Model {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        Self::parse(&text)
    }

    /// Draw with `m` as the model matrix (GL `glPushMatrix`/`glTranslatef`/`glRotatef` stack).
    pub fn draw(&self, tex: &Texture2D, m: &Mat4, tint: Color) {
        self.draw_driving(tex, m, tint, 0.0, 0.0);
    }

    /// Imported wheel objects rotate around their own axle; front wheels also steer.
    pub fn draw_driving(&self, tex: &Texture2D, m: &Mat4, tint: Color, spin: f32, steering: f32) {
        Self::draw_data(&self.pos, &self.uv, &self.cache, tex, m, tint);
        for wheel in &self.wheels {
            let wm = *m * wheel_transform(wheel.pivot, spin, if wheel.front { steering } else { 0.0 });
            Self::draw_data(&wheel.pos, &wheel.uv, &wheel.cache, tex, &wm, tint);
        }
    }

    fn draw_data(pos: &[[f32; 3]], uv: &[[f32; 2]], cache: &RefCell<Option<CachedGeometry>>, tex: &Texture2D, m: &Mat4, tint: Color) {
        const CHUNK: usize = 4998; // macroquad's batch holds 5000 indices
        let color = Vertex::new2(Vec3::ZERO, Vec2::ZERO, tint).color;
        let mut cached = cache.borrow_mut();
        let geometry = cached.get_or_insert_with(|| {
            let mut chunks = Vec::new();
            for start in (0..pos.len()).step_by(CHUNK) {
                let end = (start + CHUNK).min(pos.len());
                let vertices: Vec<_> = (start..end).map(|i| Vertex::new2(Vec3::from_array(pos[i]), Vec2::from_array(uv[i]), tint)).collect();
                let indices = (0..vertices.len() as u16).collect();
                chunks.push(Mesh { vertices, indices, texture: None });
            }
            CachedGeometry { chunks, color, matrix: None }
        });
        // Keep world-space vertices so Macroquad can batch different city
        // instances together. A separate GPU Model matrix for each traffic
        // light/road would break those batches into many small draw calls.
        // Reuse the buffers/indices and skip transforms for unchanged instances.
        let recolor = geometry.color != color;
        let transform = geometry.matrix != Some(*m);
        for (i, chunk) in geometry.chunks.iter_mut().enumerate() {
            if transform {
                for (j, vertex) in chunk.vertices.iter_mut().enumerate() {
                    vertex.position = m.transform_point3(Vec3::from_array(pos[i * CHUNK + j]));
                }
            }
            if recolor { for vertex in &mut chunk.vertices { vertex.color = color; } }
            chunk.texture = Some(tex.clone());
            draw_mesh(chunk);
        }
        geometry.color = color;
        geometry.matrix = Some(*m);
    }
}

/// `glTranslatef(x,y,z)` then `glRotatef(deg, 0,1,0)`.
pub fn at(x: f32, y: f32, z: f32, yaw_deg: f32) -> Mat4 {
    Mat4::from_translation(vec3(x, y, z)) * Mat4::from_rotation_y(yaw_deg.to_radians())
}

#[cfg(test)]
mod import_tests {
    use super::*;
    const OBJ: &str = "v 0 0 0\nv 4 0 0\nv 0 2 8\nvt 0 0\nvt 1 0\nvt 0 1\nf -3/1 -2/2 -1/3\n";

    #[test]
    fn imported_model_fits_stock_footprint_and_preserves_uvs() {
        let model = Model::import_obj(OBJ, 1.0, 2.0, 2.0).unwrap();
        assert_eq!(model.pos, vec![[-1.0, 0.0, -2.0], [1.0, 0.0, -2.0], [-1.0, 1.0, 2.0]]);
        assert_eq!(model.uv, vec![[0.0, 1.0], [1.0, 1.0], [0.0, 0.0]]);
    }

    #[test]
    fn malformed_models_are_rejected() {
        for text in ["", &OBJ.replace("4 0 0", "NaN 0 0"), &OBJ.replace("-3/1", "99/1"),
            &OBJ.replace("-3/1", "-3/0"), &OBJ.replace("-3/1", "-3"),
            &OBJ.replace("f -3/1 -2/2 -1/3", "f -3/1 -2/2 -1/3 -1/3")] {
            assert!(Model::import_obj(text, 1.0, 2.0, 2.0).is_err());
        }
    }

    fn rigged_obj() -> String {
        let mut obj = "vt 0 0\n".to_string();
        let mut vertex = 1;
        for (name, x, z) in [("body",0.0,0.0),("wheel_fl",1.0,1.2),("wheel_fr",-1.0,1.2),("wheel_rl",1.0,-1.2),("wheel_rr",-1.0,-1.2)] {
            obj.push_str(&format!("o {name}\n"));
            let points = if name == "body" { vec![[0.0,1.0,-2.0],[0.0,1.0,2.0],[1.0,1.4,0.0]] }
                else { vec![[x,0.5,z],[x,0.0,z-0.3],[x,0.0,z+0.3],[x,0.5,z],[x+0.1,0.0,z-0.3],[x+0.1,0.0,z+0.3]] };
            for triangle in points.chunks(3) {
                for p in triangle { obj.push_str(&format!("v {} {} {}\n",p[0],p[1],p[2])); }
                obj.push_str(&format!("f {}/1 {}/1 {}/1\n",vertex,vertex+1,vertex+2));
                vertex += 3;
            }
        }
        obj
    }

    #[test]
    fn named_wheels_are_separated_without_losing_faces() {
        let model = Model::import_obj(&rigged_obj(), 1.5, 2.0, 2.0).unwrap();
        assert_eq!(model.wheel_count(), 4);
        assert_eq!(model.pos.len(), 3);
        assert_eq!(model.wheels.iter().map(|w|w.pos.len()).sum::<usize>(), 24);
        assert!(model.wheels[..2].iter().all(|w|w.front));
        assert!(model.wheels[2..].iter().all(|w|!w.front));
        assert!(Model::import_obj(&rigged_obj().replace("wheel_rr","body"),1.5,2.0,2.0).is_err());
    }

    #[test]
    fn wheel_motion_keeps_the_axle_fixed_and_reverses_spin() {
        let pivot = vec3(0.8,0.3,1.2);
        let forward = wheel_transform(pivot, std::f32::consts::FRAC_PI_2, 0.0);
        let reverse = wheel_transform(pivot, -std::f32::consts::FRAC_PI_2, 0.0);
        assert!((forward.transform_point3(pivot)-pivot).length()<1e-6);
        assert!((wheel_transform(pivot,2.0,0.4).transform_point3(pivot)-pivot).length()<1e-6);
        assert!((wheel_transform(pivot,0.0,0.0).transform_point3(Vec3::ONE)-Vec3::ONE).length()<1e-6);
        let top = pivot + Vec3::Y * 0.3;
        assert!(forward.transform_point3(top).z > pivot.z);
        assert!(reverse.transform_point3(top).z < pivot.z);
    }
}
