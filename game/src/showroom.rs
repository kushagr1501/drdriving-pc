//! A close-up, orbitable studio view of the same car used on the road.
use crate::{
    game::{sp, Game},
    sprites as S,
};
use drdriving_core::cars::car;
use macroquad::prelude::*;

impl Game {
    pub fn process_showroom(&mut self) {
        if self.keys.pressed(KeyCode::Escape) || self.pads.get_clicked(sp::LIST_BACK) {
            self.enter_mode(9);
            return;
        }
        let direction = if self.pads.get_clicked(sp::LIST_PREV) || self.keys.pressed(KeyCode::Left)
        {
            -1
        } else if self.pads.get_clicked(sp::LIST_NEXT) || self.keys.pressed(KeyCode::Right) {
            1
        } else {
            0
        };
        if direction != 0 {
            let order = self.catalog_order();
            let current = order.iter().position(|&c| c == self.cargo.car).unwrap_or(0) as i32;
            self.cargo.car = order[(current + direction).rem_euclid(order.len() as i32) as usize];
        }
        if is_mouse_button_down(MouseButton::Left)
            && mouse_position().1 > 105.0
            && mouse_position().1 < screen_height() - 100.0
        {
            let d = mouse_delta_position();
            self.showroom_yaw -= d.x * 3.2;
            self.showroom_pitch = (self.showroom_pitch + d.y * 2.0).clamp(0.04, 0.65);
        }
        if self.keys.down(KeyCode::A) {
            self.showroom_yaw -= self.dt * 0.9;
        }
        if self.keys.down(KeyCode::D) {
            self.showroom_yaw += self.dt * 0.9;
        }
        if self.keys.down(KeyCode::W) {
            self.showroom_pitch = (self.showroom_pitch + self.dt * 0.4).min(0.65);
        }
        if self.keys.down(KeyCode::S) {
            self.showroom_pitch = (self.showroom_pitch - self.dt * 0.4).max(0.04);
        }
        self.showroom_distance = (self.showroom_distance - mouse_wheel().1 * 0.3).clamp(4.5, 10.0);
        if self.keys.down(KeyCode::Equal) {
            self.showroom_distance = (self.showroom_distance - self.dt * 2.0).max(4.5);
        }
        if self.keys.down(KeyCode::Minus) {
            self.showroom_distance = (self.showroom_distance + self.dt * 2.0).min(10.0);
        }
        if self.keys.pressed(KeyCode::R) {
            self.showroom_yaw = -0.52;
            self.showroom_pitch = 0.16;
            self.showroom_distance = 6.8;
        }
    }

    pub fn render_showroom(&mut self) {
        clear_background(Color::new(0.075, 0.085, 0.105, 1.0));
        let c = self.cargo.car;
        let target = vec3(0.0, 0.75, 0.0);
        let aspect = screen_width() / screen_height();
        let framing = (1.3 / aspect).max(1.0);
        let position = target
            + vec3(
                self.showroom_yaw.sin() * self.showroom_pitch.cos(),
                self.showroom_pitch.sin(),
                self.showroom_yaw.cos() * self.showroom_pitch.cos(),
            ) * self.showroom_distance
                * framing;
        let cam = Camera3D {
            position,
            target,
            up: Vec3::Y,
            fovy: 34.0_f32.to_radians(),
            z_near: 1.0,
            z_far: 65.0,
            aspect: Some(aspect),
            ..Default::default()
        };
        set_camera(&cam);
        gl_use_material(&self.a.mats.normal);
        draw_plane(
            vec3(0.0, -0.03, 0.0),
            vec2(100.0, 100.0),
            None,
            Color::new(0.13, 0.15, 0.18, 1.0),
        );
        // A continuous studio floor keeps attention on the car silhouette.
        let m = Mat4::IDENTITY;
        if let Some(detailed) = &self.a.detailed_cars[c] {
            self.a
                .car_renderer
                .shadow(m, detailed.width, detailed.length);
            detailed.draw(
                &self.a.car_renderer,
                m,
                position,
                self.preview_wheel_phase.to_radians(),
                0.0,
                1.0,
            );
        } else {
            self.a.car_model[c].draw_driving(
                &self.a.car_tex[c],
                &m,
                WHITE,
                self.preview_wheel_phase.to_radians(),
                0.0,
            );
            if !self.a.custom_car[c] {
                let info = car(c);
                let scale = Mat4::from_scale(vec3(
                    info.wheel_width_ratio,
                    info.wheel_height,
                    info.wheel_height,
                ));
                for z in [info.wheel_front, -info.wheel_back] {
                    let wheel = Mat4::from_translation(vec3(0.0, info.wheel_height, z))
                        * Mat4::from_rotation_x(self.preview_wheel_phase.to_radians())
                        * scale;
                    self.a.road_model[7].draw(&self.a.road_tex[7], &wheel, WHITE);
                }
            }
        }
        gl_use_default_material();
        set_default_camera();
        let g = &mut self.g;
        g.set_tint(Color::new(0.035, 0.045, 0.06, 0.96));
        g.e(&self.a.ui, S::WHITE_DOT, 400.0, 42.0, g.vw, 42.0);
        g.clear_tint();
        let name = self.a.custom_car_name[c].as_deref().unwrap_or(car(c).name);
        self.text.e(g, name, 400.0, 33.0, 0, 30.0, 600.0);
        g.set_tint(Color::new(0.67, 0.72, 0.78, 1.0));
        self.text.e(
            g,
            "Drag to orbit  /  Scroll to zoom  /  A D W S to rotate  /  R to reset",
            400.0,
            65.0,
            0,
            14.0,
            720.0,
        );
        g.clear_tint();
        for (id, label) in [
            (sp::LIST_PREV, "Previous car"),
            (sp::LIST_BACK, "Back to garage"),
            (sp::LIST_NEXT, "Next car"),
        ] {
            let p = &self.pads.p[id];
            g.set_tint(Color::new(0.04, 0.06, 0.085, 0.98));
            g.e(&self.a.ui, S::WHITE_DOT, p.x, p.y, 95.0, 22.0);
            g.clear_tint();
            self.text.e(g, label, p.x, p.y, 0, 18.0, 92.0);
        }
    }
}
