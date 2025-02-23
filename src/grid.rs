use bevy::{prelude::*, render::primitives::Aabb};
use std::collections::HashMap;

use crate::{
    cell::Cell,
    components::{Destination, RtsDynamicObj, RtsObj, RtsObjSize},
    events::UpdateCostEv,
    utils,
};

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Grid>()
            .add_systems(PostStartup, initialize_costfield)
            .add_systems(Update, (update_costfield_on_add, add))
            .add_observer(update_costfield_on_remove)
            .add_observer(remove);

        app.add_systems(Update, print_occupied_cells.run_if(resource_exists::<Grid>));
    }
}

fn print_occupied_cells(grid: Res<Grid>) {
    for (_ent, cells) in grid.occupied_cells.iter() {
        for cell in cells.iter() {
            // print!("-{},{}", cell.y, cell.x);
        }
    }

    // println!();
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct Grid {
    pub cell_radius: f32,
    pub cell_diameter: f32,
    pub grid: Vec<Vec<Cell>>,
    pub size: IVec2, // 'x' represents rows, 'y' represents columns
    pub occupied_cells: HashMap<u32, Vec<IVec2>>,
}

impl Grid {
    // creates the grid and the costfield
    // all flowfields will share the same costfield
    pub fn new(size: IVec2, cell_diameter: f32) -> Self {
        let mut grid = Grid {
            cell_diameter,
            cell_radius: cell_diameter / 2.0,
            grid: Vec::default(),
            size,
            occupied_cells: HashMap::default(),
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

        grid
    }

    pub fn get_cell_from_world_position(&self, world_pos: Vec3) -> Cell {
        // Calculate the offset for the grid's top-left corner
        let adjusted_x = world_pos.x - (-self.size.x as f32 * self.cell_diameter / 2.0);
        let adjusted_y = world_pos.z - (-self.size.y as f32 * self.cell_diameter / 2.0);

        // Calculate percentages within the grid
        let percent_x = adjusted_x / (self.size.x as f32 * self.cell_diameter);
        let percent_y = adjusted_y / (self.size.y as f32 * self.cell_diameter);

        let offset = Some(Vec2::new(percent_x, percent_y));

        utils::get_cell_from_world_position_helper(
            world_pos,
            self.size,
            self.cell_diameter,
            &self.grid,
            offset,
        )
    }

    pub fn update_cell_costs(
        &mut self,
        entity_id: u32,
        obj_transform: &Transform,
        obj_size: &RtsObjSize,
    ) {
        self.for_each_cell_in_obj(entity_id, obj_transform, obj_size, |grid, pos, cells| {
            grid.update_cell_cost_helper(pos, cells);
        });
    }

    pub fn reset_cell_costs(
        &mut self,
        entity_id: u32,
        obj_transform: &Transform,
        obj_size: &RtsObjSize,
    ) {
        self.for_each_cell_in_obj(entity_id, obj_transform, obj_size, |grid, pos, cells| {
            grid.reset_cell_cost_helper(pos, cells);
        });
    }

    // Iterates over all grid cell positions that intersect with the unit’s AABB.
    fn for_each_cell_in_obj<F>(
        &mut self,
        entity_id: u32,
        obj_transform: &Transform,
        obj_size: &RtsObjSize,
        mut callback: F,
    ) where
        F: FnMut(&mut Self, Vec3, Vec<IVec2>),
    {
        let cell_size = self.cell_diameter;
        let grid_offset_x = -self.size.x as f32 * cell_size / 2.0;
        let grid_offset_y = -self.size.y as f32 * cell_size / 2.0;

        let obj_pos = obj_transform.translation;
        let half_extent = obj_size.0 / 2.0;

        let aabb = Aabb::from_min_max(
            Vec3::new(
                obj_pos.x - half_extent.x,
                obj_pos.y - half_extent.y,
                obj_pos.z - half_extent.y,
            ),
            Vec3::new(
                obj_pos.x + half_extent.x,
                obj_pos.y + half_extent.y,
                obj_pos.z + half_extent.y,
            ),
        );

        let min_x = ((aabb.min().x - grid_offset_x) / cell_size).floor() as isize;
        let max_x = ((aabb.max().x - grid_offset_x) / cell_size).floor() as isize;
        let min_y = ((aabb.min().z - grid_offset_y) / cell_size).floor() as isize;
        let max_y = ((aabb.max().z - grid_offset_y) / cell_size).floor() as isize;

        let mut occupied_cells = Vec::new();
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                if x >= 0 && x < self.size.x as isize && y >= 0 && y < self.size.y as isize {
                    occupied_cells.push(IVec2::new(x as i32, y as i32));

                    let cell_pos = Vec3::new(
                        x as f32 * cell_size + grid_offset_x,
                        0.0,
                        y as f32 * cell_size + grid_offset_y,
                    );
                }
            }
        }

        callback(self, Vec3::ZERO, occupied_cells);
        // self.occupied_cells.insert(entity_id, occupied_cells);
    }

    fn update_cell_cost_helper(&mut self, position: Vec3, cells: Vec<IVec2>) -> Cell {
        let cell = self.get_cell_from_world_position(position);

        for cell in cells.iter() {
            self.grid[cell.y as usize][cell.x as usize].cost = 255;
        }

        // if cell.idx.y < self.grid.len() as i32
        //     && cell.idx.x < self.grid[cell.idx.y as usize].len() as i32
        // {
        //     self.grid[cell.idx.y as usize][cell.idx.x as usize].cost = 255;
        // }
        cell
    }

    // TODO: Will eventually need rework. This is setting the cell cost back to 1. What if the cost was originally
    // something else? Like different terrain (mud, snow)? Maybe we need to store the original costfield in a hashmap or something
    fn reset_cell_cost_helper(&mut self, position: Vec3, cells: Vec<IVec2>) -> Cell {
        let cell = self.get_cell_from_world_position(position);

        for cell in cells.iter() {
            self.grid[cell.y as usize][cell.x as usize].cost = 1;
        }

        // self.grid[cell.idx.y as usize][cell.idx.x as usize].cost = 1;
        cell
    }
}

// update this so that it gets the aabb of the entity and checks if it intersects with the cell
fn initialize_costfield(
    mut grid: ResMut<Grid>,
    q_objects: Query<(Entity, &Transform, &RtsObjSize), With<RtsObj>>,
) {
    let objects = q_objects.iter().collect::<Vec<_>>();

    for (ent, transform, size) in objects {
        grid.update_cell_costs(ent.index(), transform, size);
    }
}

// detects if a new static object has been added and updates the costfield
fn update_costfield_on_add(
    mut cmds: Commands,
    mut grid: ResMut<Grid>,
    q_objects: Query<(Entity, &Transform, &RtsObjSize), Added<RtsObj>>,
) {
    let objects = q_objects.iter().collect::<Vec<_>>();
    if objects.is_empty() {
        return;
    }

    for (ent, transform, size) in objects.iter() {
        grid.update_cell_costs(ent.index(), transform, size);
    }

    cmds.trigger(UpdateCostEv);
}

fn update_costfield_on_remove(
    trigger: Trigger<OnRemove, RtsObj>,
    mut cmds: Commands,
    mut grid: ResMut<Grid>,
    q_transform: Query<(Entity, &Transform, &RtsObjSize)>,
) {
    let ent = trigger.entity();
    if let Ok((ent, transform, size)) = q_transform.get(ent) {
        grid.reset_cell_costs(ent.index(), transform, size);
    } else {
        return;
    }

    cmds.trigger(UpdateCostEv);
}

fn add(
    mut cmds: Commands,
    mut grid: ResMut<Grid>,
    q_units: Query<(Entity, &Transform, &RtsObjSize), Added<Destination>>,
) {
    let units = q_units.iter().collect::<Vec<_>>();
    if units.is_empty() {
        return;
    }

    for (ent, transform, size) in units.iter() {
        grid.update_cell_costs(ent.index(), transform, size);
        cmds.entity(*ent).remove::<RtsObj>();
    }

    cmds.trigger(UpdateCostEv);
}

fn remove(
    trigger: Trigger<OnRemove, Destination>,
    mut cmds: Commands,
    mut grid: ResMut<Grid>,
    q_transform: Query<(Entity, &Transform, &RtsObjSize)>,
) {
    let ent = trigger.entity();
    if let Ok((ent, transform, size)) = q_transform.get(ent) {
        grid.reset_cell_costs(ent.index(), transform, size);
        cmds.entity(ent).insert(RtsObj);
    } else {
        return;
    }

    cmds.trigger(UpdateCostEv);
}

// TODO: remove?
// update this so that it gets the aabb of the entity and checks if it intersects with the cell
fn _update_costfield_og(
    // mut cmds: Commands,
    grid: Res<Grid>,
    q: Query<(&Transform, Entity), Added<RtsDynamicObj>>,
) {
    // For each newly added dynamic object, compute an AABB in the XZ plane.
    for (transform, _entity) in q.iter() {
        // Assume a default half-extent for the entity's AABB; adjust as needed.
        let half_extent = 0.5;
        let entity_min = Vec2::new(
            transform.translation.x - half_extent,
            transform.translation.z - half_extent,
        );
        let entity_max = Vec2::new(
            transform.translation.x + half_extent,
            transform.translation.z + half_extent,
        );

        // Iterate through all grid cells.
        // Each cell is assumed to be a square centered at its world_pos,
        // with half size grid.cell_radius.
        for row in grid.grid.iter() {
            for cell in row.iter() {
                let cell_min = Vec2::new(
                    cell.world_pos.x - grid.cell_radius,
                    cell.world_pos.z - grid.cell_radius,
                );
                let cell_max = Vec2::new(
                    cell.world_pos.x + grid.cell_radius,
                    cell.world_pos.z + grid.cell_radius,
                );

                // Check if the entity's AABB intersects with the cell's AABB.
                if entity_min.x <= cell_max.x
                    && entity_max.x >= cell_min.x
                    && entity_min.y <= cell_max.y
                    && entity_max.y >= cell_min.y
                {
                    // cmds.trigger(UpdateCostEv::new(*cell));
                }
            }
        }
    }
}
