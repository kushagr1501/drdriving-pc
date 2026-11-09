//! Direct car selection, with installed named cars first and all stock cars after them.
use crate::{game::{sp, Game, COLOR_GREEN}, sprites as S};
use drdriving_core::cars::{car, PLAYER_CARS};
use macroquad::prelude::*;

impl Game {
    pub fn catalog_order(&self) -> Vec<usize> {
        let mut cars: Vec<usize> = (0..PLAYER_CARS).collect();
        cars.sort_by_key(|&c| (!self.a.custom_car[c], c));
        cars
    }

    pub fn open_car_catalog(&mut self) {
        self.catalog_return = self.mode;
        self.catalog_page = 0;
        self.enter_mode(16);
    }

    pub fn process_car_catalog(&mut self) {
        let cars = self.catalog_order();
        let pages = cars.len().div_ceil(8);
        if self.pads.get_clicked(sp::LIST_BACK) || self.keys.pressed(KeyCode::Escape) {
            self.enter_mode(self.catalog_return);
            return;
        }
        if self.pads.get_clicked(sp::LIST_PREV) || self.keys.pressed(KeyCode::Left) {
            self.catalog_page = self.catalog_page.saturating_sub(1);
        }
        if self.pads.get_clicked(sp::LIST_NEXT) || self.keys.pressed(KeyCode::Right) {
            self.catalog_page = (self.catalog_page + 1).min(pages - 1);
        }
        for (row, key) in [KeyCode::Key1,KeyCode::Key2,KeyCode::Key3,KeyCode::Key4,KeyCode::Key5,KeyCode::Key6,KeyCode::Key7,KeyCode::Key8].iter().enumerate() {
            if self.pads.get_clicked(sp::LIST_ROW + row) || self.keys.pressed(*key) {
                if let Some(&c) = cars.get(self.catalog_page * 8 + row) {
                    self.cargo.car = c;
                    let owned = self.save.has_car[c] || self.save.car_rent[c] > 0;
                    if owned { self.world.car_mode = c; }
                    self.enter_mode(if self.catalog_return == 1 && owned { 1 } else { 9 });
                    return;
                }
            }
        }
    }

    pub fn render_car_catalog(&mut self) {
        self.render_background(0);
        let cars = self.catalog_order();
        let pages = cars.len().div_ceil(8);
        let page = self.catalog_page.min(pages - 1);
        let selected = if self.catalog_return == 1 { self.world.car_mode } else { self.cargo.car };
        let g = &mut self.g;
        self.text.e(g, "Choose a car", 400.0, 67.0, 0, 27.0, 300.0);
        self.text.e(g, &format!("{} cars  /  page {} of {}", cars.len(), page + 1, pages), 400.0, 93.0, 0, 15.0, 300.0);
        for (row, &c) in cars.iter().skip(page * 8).take(8).enumerate() {
            let pad = &self.pads.p[sp::LIST_ROW + row];
            let picked = c == selected;
            g.set_tint(if picked { Color::new(0.13,0.30,0.12,0.96) } else { Color::new(0.04,0.07,0.09,0.94) });
            g.e(&self.a.ui, S::WHITE_DOT, 400.0, pad.y, 310.0, 22.0);
            g.clear_tint();
            let name = self.a.custom_car_name[c].as_deref().unwrap_or(car(c).name);
            self.text.e(g, &format!("{}", row + 1), 115.0, pad.y, 0, 17.0, 20.0);
            self.text.e(g, name, 145.0, pad.y, 1, 20.0, 195.0);
            let owned = self.save.has_car[c] || self.save.car_rent[c] > 0;
            g.set_tint(if owned { COLOR_GREEN } else { GRAY });
            self.text.e(g, if picked { "Selected" } else if owned { "Ready" } else { "Locked" }, 690.0, pad.y, 2, 15.0, 75.0);
            g.clear_tint();
        }
        for (id, label) in [(sp::LIST_PREV, "Previous"), (sp::LIST_BACK, "Back"), (sp::LIST_NEXT, "Next")] {
            let pad = &self.pads.p[id];
            g.set_tint(Color::new(0.06,0.12,0.16,0.98));
            g.e(&self.a.ui, S::WHITE_DOT, pad.x, pad.y, 85.0, 22.0);
            g.clear_tint();
            self.text.e(g, label, pad.x, pad.y, 0, 19.0, 80.0);
        }
    }
}
