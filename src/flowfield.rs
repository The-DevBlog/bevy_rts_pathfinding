use crate::{
    cell::*, grid::Grid, grid_direction::GridDirection, utils, GameCamera, InitializeFlowFieldEv,
    MapBase, Selected, SetActiveFlowfieldEv,
};
use bevy::{prelude::*, render::primitives::Aabb, window::PrimaryWindow};
use bevy_rapier3d::prelude::Collider;
use std::{cmp::min, collections::VecDeque};

pub struct FlowfieldPlugin;

impl Plugin for FlowfieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(initialize_flowfield);
    }
}

#[derive(Component, Clone, Default, PartialEq)]
pub struct FlowField {
    pub cell_radius: f32,
    pub cell_diameter: f32,
    pub destination_cell: Cell,
    pub grid: Vec<Vec<Cell>>,
    pub size: IVec2,
    pub units: Vec<Entity>,
}

impl FlowField {
    pub fn new(cell_radius: f32, grid_size: IVec2, selected_units: Vec<Entity>) -> Self {
        FlowField {
            cell_radius,
            cell_diameter: cell_radius * 2.,
            destination_cell: Cell::default(),
            grid: Vec::default(),
            size: grid_size,
            units: selected_units,
        }
    }

    pub fn create_integration_field(&mut self, mut grid: ResMut<Grid>, destination_cell: Cell) {
        // println!("Start Integration Field Create");

        self.grid = grid.grid.clone();

        // Initialize the destination cell in the grid
        let dest_idx = destination_cell.idx;
        let dest_cell = &mut self.grid[dest_idx.y as usize][dest_idx.x as usize];
        dest_cell.cost = 0;
        dest_cell.best_cost = 0;
        self.destination_cell = dest_cell.clone();

        let mut cells_to_check: VecDeque<IVec2> = VecDeque::new();
        cells_to_check.push_back(dest_idx);

        while let Some(cur_idx) = cells_to_check.pop_front() {
            let cur_x = cur_idx.x as usize;
            let cur_y = cur_idx.y as usize;

            let cur_cell_best_cost = self.grid[cur_y][cur_x].best_cost;

            // Iterate over cardinal directions
            for direction in GridDirection::cardinal_directions() {
                let delta = direction.vector();
                let neighbor_idx = cur_idx + delta;

                if neighbor_idx.x >= 0
                    && neighbor_idx.x < self.size.x
                    && neighbor_idx.y >= 0
                    && neighbor_idx.y < self.size.y
                {
                    let neighbor_x = neighbor_idx.x as usize;
                    let neighbor_y = neighbor_idx.y as usize;

                    let neighbor_cell = &mut self.grid[neighbor_y][neighbor_x];

                    if neighbor_cell.cost == u8::MAX {
                        continue;
                    }

                    let tentative_best_cost = neighbor_cell.cost as u16 + cur_cell_best_cost;
                    if tentative_best_cost < neighbor_cell.best_cost {
                        neighbor_cell.best_cost = tentative_best_cost;
                        cells_to_check.push_back(neighbor_idx);
                    }
                }
            }
        }

        // println!("End Integration Field Create");
    }

    pub fn create_flowfield(&mut self) {
        // println!("Start Flowfield Create");

        let grid_size_y = self.size.y as usize;
        let grid_size_x = self.size.x as usize;

        for y in 0..grid_size_y {
            for x in 0..grid_size_x {
                let cell = &self.grid[y][x]; // Immutable borrow to get best_cost
                let mut best_cost = cell.best_cost;
                let mut best_direction = GridDirection::None;

                // Get all possible directions
                for direction in GridDirection::all_directions() {
                    let delta = direction.vector();
                    let nx = x as isize + delta.x as isize;
                    let ny = y as isize + delta.y as isize;

                    if nx >= 0 && nx < grid_size_x as isize && ny >= 0 && ny < grid_size_y as isize
                    {
                        let neighbor = &self.grid[ny as usize][nx as usize];
                        if neighbor.best_cost < best_cost {
                            best_cost = neighbor.best_cost;
                            best_direction = direction;
                        }
                    }
                }

                // Now, set the best_direction for the cell
                self.grid[y][x].best_direction = best_direction;
            }
        }
    }

    pub fn get_cell_from_world_position(&self, world_pos: Vec3) -> Cell {
        // Adjust world position relative to the grid's top-left corner
        let adjusted_x = world_pos.x - (-self.size.x as f32 * self.cell_diameter / 2.0);
        let adjusted_y = world_pos.z - (-self.size.y as f32 * self.cell_diameter / 2.0);

        // Calculate percentages within the grid
        let mut percent_x = adjusted_x / (self.size.x as f32 * self.cell_diameter);
        let mut percent_y = adjusted_y / (self.size.y as f32 * self.cell_diameter);

        // Clamp percentages to ensure they're within [0.0, 1.0]
        percent_x = percent_x.clamp(0.0, 1.0);
        percent_y = percent_y.clamp(0.0, 1.0);

        // Calculate grid indices
        let x = ((self.size.x as f32) * percent_x).floor() as usize;
        let y = ((self.size.y as f32) * percent_y).floor() as usize;

        let x = min(x, self.size.x as usize - 1);
        let y = min(y, self.size.y as usize - 1);

        self.grid[y][x] // Swap x and y
    }
}

fn initialize_flowfield(
    _trigger: Trigger<InitializeFlowFieldEv>,
    mut cmds: Commands,
    mut grid: ResMut<Grid>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_cam: Query<(&Camera, &GlobalTransform), With<GameCamera>>,
    q_map_base: Query<&GlobalTransform, With<MapBase>>,
    q_selected: Query<(&Transform, &Collider, Entity), With<Selected>>,
) {
    // println!("Start Initialize Flowfield");

    let Some(mouse_pos) = q_windows.single().cursor_position() else {
        return;
    };

    let Ok(cam) = q_cam.get_single() else {
        return;
    };

    let Ok(map_base) = q_map_base.get_single() else {
        return;
    };

    // let selected_units: Vec<Entity> = q_selected.iter().collect();
    let mut selected_units = Vec::new();
    let mut unit_positions = Vec::new();
    for (unit_transform, collider, unit_entity) in q_selected.iter() {
        let size = collider.as_cuboid().unwrap().half_extents() * 0.5;
        selected_units.push(unit_entity);
        unit_positions.push((unit_transform.translation, (size.x, size.z)));
    }

    grid.reset_selected_unit_costs(unit_positions);

    let world_mouse_pos = utils::get_world_pos(map_base, cam.1, cam.0, mouse_pos);
    let destination_cell = grid.get_cell_from_world_position(world_mouse_pos);

    let mut flowfield = FlowField::new(grid.cell_radius, grid.size, selected_units);
    flowfield.create_integration_field(grid, destination_cell);
    flowfield.create_flowfield();

    cmds.trigger(SetActiveFlowfieldEv(Some(flowfield.clone())));
    cmds.spawn(flowfield);

    // println!("End Initialize Flowfield");
}
