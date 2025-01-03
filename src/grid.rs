use crate::{cell::Cell, components::Destination, utils, UpdateCostEv};

use bevy::prelude::*;
use std::collections::HashSet;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Grid>()
            .init_resource::<OccupiedCells>()
            .add_event::<UpdateCostEv>()
            .add_systems(Update, update_costs);
    }
}

#[derive(Resource, Default)]
pub struct OccupiedCells(HashSet<IVec2>);

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct Grid {
    pub size: IVec2,
    pub cell_radius: f32,
    pub cell_diameter: f32,
    pub grid: Vec<Vec<Cell>>,
}

impl Grid {
    // creates the grid and the costfield
    // all flowfields will share the same costfield
    pub fn new<F>(size: IVec2, cell_diameter: f32, mut collision_checker: F) -> Self
    where
        F: FnMut(Vec3) -> bool,
    {
        let mut grid = Grid {
            size,
            cell_diameter,
            cell_radius: cell_diameter / 2.0,
            grid: Vec::default(),
        };

        // Calculate offsets for top-left alignment
        let offset_x = -(grid.size.x as f32 * grid.cell_diameter) / 2.;
        let offset_y = -(grid.size.y as f32 * grid.cell_diameter) / 2.;

        // Initialize Grid
        grid.grid = (0..grid.size.y)
            .map(|y| {
                (0..grid.size.x)
                    .map(|x| {
                        let x_pos = grid.cell_diameter * x as f32 + grid.cell_radius + offset_x;
                        let y_pos = grid.cell_diameter * y as f32 + grid.cell_radius + offset_y;
                        let world_pos = Vec3::new(x_pos, 0.0, y_pos);
                        Cell::new(world_pos, IVec2::new(x, y))
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // Create Costfield
        for y in 0..grid.size.y {
            for x in 0..grid.size.x {
                let world_pos = grid.grid[y as usize][x as usize].world_pos;

                if collision_checker(world_pos) {
                    grid.grid[y as usize][x as usize].increase_cost(255);
                }
            }
        }

        grid
    }

    pub fn get_cell_from_world_position(&self, world_pos: Vec3) -> Cell {
        let cell = utils::get_cell_from_world_position_helper(
            world_pos,
            self.size,
            self.cell_diameter,
            &self.grid,
        );

        return cell;
    }

    pub fn reset_costs(&mut self, units: Vec<(Vec3, Vec2)>) {
        for (unit_pos, unit_size) in units.iter() {
            let hw = unit_size.x;
            let hh = unit_size.y;

            let min_world = Vec3::new(unit_pos.x - hw, 0.0, unit_pos.y - hh);
            let max_world = Vec3::new(unit_pos.x + hw, 0.0, unit_pos.y + hh);

            let min_cell = self.get_cell_from_world_position(min_world);
            let max_cell = self.get_cell_from_world_position(max_world);

            let min_x = min_cell.idx.x.clamp(0, self.size.x as i32 - 1);
            let max_x = max_cell.idx.x.clamp(0, self.size.x as i32 - 1);
            let min_y = min_cell.idx.y.clamp(0, self.size.y as i32 - 1);
            let max_y = max_cell.idx.y.clamp(0, self.size.y as i32 - 1);

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    self.grid[y as usize][x as usize].cost = 1;
                }
            }
        }
    }

    pub fn update_unit_cell_costs(&mut self, position: Vec3) -> Cell {
        // Determine which cell the unit occupies
        let cell = self.get_cell_from_world_position(position);

        // Set the cost of the cell to 255
        if cell.idx.y < self.grid.len() as i32
            && cell.idx.x < self.grid[cell.idx.y as usize].len() as i32
        {
            self.grid[cell.idx.y as usize][cell.idx.x as usize].cost = 255;
        }

        return cell;
    }
}

pub fn update_costs(
    mut grid: ResMut<Grid>,
    mut events: EventWriter<UpdateCostEv>,
    mut occupied_cells: ResMut<OccupiedCells>,
    q_units: Query<&Transform, With<Destination>>,
) {
    if q_units.is_empty() {
        return;
    }

    println!("updating costs");
    let mut current_occupied = HashSet::new();

    // Mark cells occupied by units
    for transform in q_units.iter() {
        let cell = grid.update_unit_cell_costs(transform.translation);
        current_occupied.insert(cell.idx);
        events.send(UpdateCostEv::new(cell)); // Send event for occupied cell
    }

    // Reset previously occupied cells that are no longer occupied
    for idx in occupied_cells.0.difference(&current_occupied) {
        if idx.y >= 0 && idx.y < grid.size.y && idx.x >= 0 && idx.x < grid.size.x {
            let cell = &mut grid.grid[idx.y as usize][idx.x as usize];
            cell.cost = 1;

            // Send event for cell reset to cost 1
            events.send(UpdateCostEv::new(*cell));
        }
    }

    // Update the occupied cells set
    occupied_cells.0 = current_occupied;
}
