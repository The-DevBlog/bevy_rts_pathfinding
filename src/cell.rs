use std::u16;

use bevy::prelude::*;

#[derive(Clone, Default, Copy)]
pub struct Cell {
    pub world_position: Vec3,
    pub grid_idx: IVec2,
    pub cost: u8,
    pub best_cost: u16,
}

impl Cell {
    pub fn new(world_position: Vec3, grid_idx: IVec2) -> Self {
        Cell {
            world_position,
            grid_idx,
            cost: 1,
            best_cost: u16::MAX,
        }
    }

    pub fn increase_cost(&mut self, amount: u8) {
        if self.cost == u8::MAX {
            return;
        }

        if let Some(new_cost) = self.cost.checked_add(amount) {
            self.cost = new_cost;
        } else {
            self.cost = u8::MAX;
        }
    }
}
