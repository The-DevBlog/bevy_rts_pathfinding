use bevy::prelude::*;

use crate::{cell::Cell, utils, UpdateCostEv};

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Grid>().add_event::<UpdateCostEv>();
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct Grid {
    pub size: IVec2, // 'x' represents rows, 'y' represents columns
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

    // pub fn get_cell_from_world_position(&self, world_pos: Vec3, offset: Option<Vec2>) -> Cell {
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
}

// write me a system that will check all the AABB of the entities that do NOT have a destination component

// TODO: This is not precise. It does not capture 'every' cell that a unit is currenlty intersecting with.
// pub fn update_costs(
//     mut occupied_cells: ResMut<OccupiedCells>,
//     mut grid: ResMut<Grid>,
//     mut cmds: Commands,
//     q_units: Query<(&Transform, &UnitSize), With<Unit>>,
//     costmap: Res<CostMap>,
// ) {
//     if q_units.is_empty() {
//         return;
//     }

//     let mut current_occupied = HashSet::new();

//     // Grid cell size (assumed uniform square grid)
//     let cell_size = grid.cell_diameter;

//     // Calculate the grid offset (world position of the grid's origin)
//     let grid_offset_x = -grid.size.x as f32 * cell_size / 2.0;
//     let grid_offset_y = -grid.size.y as f32 * cell_size / 2.0;

//     // Mark cells occupied by units
//     for (unit_transform, unit_size) in q_units.iter() {
//         let unit_pos = unit_transform.translation;

//         // Construct an Aabb for the unit
//         let half_extent = unit_size.0 / 2.0; // Half size of the unit
//         let aabb = Aabb::from_min_max(
//             Vec3::new(
//                 unit_pos.x - half_extent.x,
//                 unit_pos.y - half_extent.y,
//                 unit_pos.z - half_extent.y,
//             ),
//             Vec3::new(
//                 unit_pos.x + half_extent.x,
//                 unit_pos.y + half_extent.y,
//                 unit_pos.z + half_extent.y,
//             ),
//         );

//         // Map AABB to grid coordinates
//         let grid_min_x = ((aabb.min().x - grid_offset_x) / cell_size).floor() as isize;
//         let grid_max_x = ((aabb.max().x - grid_offset_x) / cell_size).floor() as isize;
//         let grid_min_y = ((aabb.min().z - grid_offset_y) / cell_size).floor() as isize;
//         let grid_max_y = ((aabb.max().z - grid_offset_y) / cell_size).floor() as isize;

//         // Iterate over all cells the unit intersects
//         // let mut idxs = Vec::new();
//         for y in grid_min_y..=grid_max_y {
//             for x in grid_min_x..=grid_max_x {
//                 if x >= 0 && x < grid.size.x as isize && y >= 0 && y < grid.size.y as isize {
//                     let cell = grid.update_unit_cell_costs(Vec3::new(
//                         x as f32 * cell_size + grid_offset_x,
//                         0.0,
//                         y as f32 * cell_size + grid_offset_y,
//                     ));

//                     // idxs.push(cell.idx);

//                     cmds.trigger(UpdateCostEv::new(cell));
//                     current_occupied.insert(cell.idx);
//                 }
//             }
//         }

//         // print!("Unit occupying cells:");
//         // for i in idxs.iter() {
//         //     print!(" {},{} -", i.y, i.x);
//         // }
//         // println!();
//     }

//     // Reset previously occupied cells that are no longer occupied
//     let columns = grid.grid.len();
//     for idx in occupied_cells.0.difference(&current_occupied) {
//         if idx.y >= 0 && idx.y < grid.size.y && idx.x >= 0 && idx.x < grid.size.x {
//             let cell = &mut grid.grid[idx.y as usize][idx.x as usize];
//             if let Some(cost) = costmap.0.get(&cell.idx_to_id(columns)) {
//                 cell.cost = *cost;
//             }

//             cmds.trigger(UpdateCostEv::new(*cell));
//         }
//     }

//     // Update the occupied cells set
//     occupied_cells.0 = current_occupied;
// }
