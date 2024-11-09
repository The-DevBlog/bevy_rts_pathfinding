use bevy::{
    color::palettes::{css::*, tailwind::CYAN_100},
    prelude::*,
};
use bevy_rapier3d::prelude::Collider;
use rand::Rng;
use std::collections::VecDeque;

const COLOR_GRID: Srgba = GRAY;
const COLOR_ARROWS: Srgba = CYAN_100;
const COLOR_OCCUPIED_CELL: Srgba = RED;
const CELL_SIZE: f32 = 10.0;
const NEIGHBOR_OFFSETS: [(isize, isize); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];

pub struct BevyRtsPathFindingPlugin;

impl Plugin for BevyRtsPathFindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                calculate_flow_field,
                calculate_flow_vectors,
                draw_flow_field,
                draw_grid,
            ),
        );
    }
}

#[derive(Resource)]
pub struct TargetCell {
    x: usize,
    z: usize,
}

impl TargetCell {
    pub fn new(cells_width: usize, cells_depth: usize) -> Self {
        let target = TargetCell {
            x: cells_width - 1,
            z: cells_depth - 1,
        };

        target
    }
}

#[derive(Clone)]
pub struct GridCell {
    pub position: Vec3,
    pub cost: f32,
    pub flow_vector: Vec3,
    pub occupied: bool,
}

#[derive(Resource)]
pub struct Grid {
    pub cells: Vec<Vec<GridCell>>,
    pub cells_width: usize,
    pub cells_depth: usize,
    pub colors: GridColors,
}

pub struct GridColors {
    pub grid: Srgba,
    pub arrows: Srgba,
    pub occupied_cells: Srgba,
}

impl Default for GridColors {
    fn default() -> Self {
        Self {
            grid: COLOR_GRID,
            arrows: COLOR_ARROWS,
            occupied_cells: COLOR_OCCUPIED_CELL,
        }
    }
}

impl Grid {
    pub fn new(cells_width: usize, cells_depth: usize) -> Self {
        let mut grid = vec![
            vec![
                GridCell {
                    position: Vec3::ZERO,
                    cost: f32::INFINITY,
                    flow_vector: Vec3::ZERO,
                    occupied: false,
                };
                cells_width
            ];
            cells_depth
        ];

        let mut rng = rand::thread_rng();

        // Calculate the offset to center the grid at (0, 0, 0)
        let grid_width = cells_width as f32 * CELL_SIZE;
        let grid_depth = cells_depth as f32 * CELL_SIZE;
        let half_grid_width = grid_width / 2.0;
        let half_grid_depth = grid_depth / 2.0;

        for x in 0..cells_width {
            for z in 0..cells_depth {
                let world_x = x as f32 * CELL_SIZE - half_grid_width + CELL_SIZE / 2.0;
                let world_z = z as f32 * CELL_SIZE - half_grid_depth + CELL_SIZE / 2.0;

                grid[x][z].position = Vec3::new(world_x, 0.0, world_z);

                // Randomly set some cells as obstacles
                if rng.gen_bool(0.1) {
                    grid[x][z].occupied = true;
                }
            }
        }

        let target = TargetCell::new(cells_width, cells_depth);
        grid[target.x][target.z].cost = 0.0;

        Grid {
            cells: grid,
            colors: GridColors::default(),
            cells_width: cells_width,
            cells_depth: cells_depth,
        }
    }
}

fn calculate_flow_field(mut grid: ResMut<Grid>, target: Res<TargetCell>) {
    let mut queue = VecDeque::new();
    queue.push_back((target.x, target.z));

    while let Some((x, z)) = queue.pop_front() {
        let current_cost = grid.cells[x][z].cost;

        for (dx, dz) in &NEIGHBOR_OFFSETS {
            let nx = x as isize + dx;
            let nz = z as isize + dz;

            if nx >= 0
                && nx < grid.cells_width as isize
                && nz >= 0
                && nz < grid.cells_depth as isize
            {
                let nx = nx as usize;
                let nz = nz as usize;

                let neighbor = &mut grid.cells[nx][nz];

                if neighbor.occupied {
                    continue;
                }

                let new_cost = current_cost + 1.0; // Assuming uniform cost

                if new_cost < neighbor.cost {
                    neighbor.cost = new_cost;
                    queue.push_back((nx, nz));
                }
            }
        }
    }
}

fn calculate_flow_vectors(mut grid: ResMut<Grid>) {
    for x in 0..grid.cells_width {
        for z in 0..grid.cells_depth {
            if grid.cells[x][z].occupied {
                continue;
            }

            let mut min_cost = grid.cells[x][z].cost;
            let mut min_direction = Vec3::ZERO;

            for (dx, dz) in &NEIGHBOR_OFFSETS {
                let nx = x as isize + dx;
                let nz = z as isize + dz;

                if nx >= 0
                    && nx < grid.cells_width as isize
                    && nz >= 0
                    && nz < grid.cells_depth as isize
                {
                    let nx = nx as usize;
                    let nz = nz as usize;

                    let neighbor = &grid.cells[nx][nz];

                    if neighbor.cost < min_cost {
                        min_cost = neighbor.cost;
                        min_direction = (neighbor.position - grid.cells[x][z].position).normalize();
                    }
                }
            }

            grid.cells[x][z].flow_vector = min_direction;
        }
    }
}

fn detect_colliders(mut grid: ResMut<Grid>, colliders: Query<(&Transform, &Collider)>) {
    for x in 0..grid.cells_width {
        for z in 0..grid.cells_depth {
            let cell = &mut grid.cells[x][z];
            cell.occupied = false; // Reset obstacle status

            let cell_min =
                cell.position - Vec3::new(CELL_SIZE / 2.0, CELL_SIZE / 2.0, CELL_SIZE / 2.0);
            let cell_max =
                cell.position + Vec3::new(CELL_SIZE / 2.0, CELL_SIZE / 2.0, CELL_SIZE / 2.0);

            for (collider_transform, collider) in colliders.iter() {
                // Compute the collider's AABB in world space
                let collider_aabb = collider.compute_aabb(&collider_transform.compute_matrix());

                // Check for overlap
                if aabb_overlap(cell_min, cell_max, collider_aabb.mins, collider_aabb.maxs) {
                    cell.occupied = true;
                    break; // No need to check other colliders for this cell
                }
            }
        }
    }
}

fn aabb_overlap(min1: Vec3, max1: Vec3, min2: Vec3, max2: Vec3) -> bool {
    (min1.x <= max2.x && max1.x >= min2.x)
        && (min1.y <= max2.y && max1.y >= min2.y)
        && (min1.z <= max2.z && max1.z >= min2.z)
}

fn draw_flow_field(grid: Res<Grid>, mut gizmos: Gizmos) {
    let arrow_len = CELL_SIZE * 0.75 / 2.0;

    for x in 0..grid.cells_width {
        for z in 0..grid.cells_depth {
            let cell = &grid.cells[x][z];

            if cell.occupied || cell.flow_vector == Vec3::ZERO {
                // Draw an 'X' for each occupied cell
                let top_left = cell.position + Vec3::new(-arrow_len, 0.0, -arrow_len);
                let top_right = cell.position + Vec3::new(arrow_len, 0.0, -arrow_len);
                let bottom_left = cell.position + Vec3::new(-arrow_len, 0.0, arrow_len);
                let bottom_right = cell.position + Vec3::new(arrow_len, 0.0, arrow_len);

                gizmos.line(top_left, bottom_right, RED);
                gizmos.line(top_right, bottom_left, RED);

                continue;
            }

            // Normalize the flow vector
            let flow_direction = cell.flow_vector.normalize();

            // Calculate start and end points
            let start = cell.position - flow_direction * arrow_len;
            let end = cell.position + flow_direction * arrow_len;

            gizmos.arrow(start, end, COLOR_ARROWS);
        }
    }
}

fn draw_grid(mut gizmos: Gizmos, grid: Res<Grid>) {
    gizmos.grid(
        Vec3::ZERO,
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        UVec2::new(grid.cells_width as u32, grid.cells_depth as u32),
        Vec2::new(CELL_SIZE, CELL_SIZE),
        COLOR_GRID,
    );
}

// GAME LOGIC
// #[derive(Component)]
// struct Agent;

// fn spawn_agents(mut commands: Commands) {
//     let agent = ();
//     for _ in 0..10 {
//         commands
//             .spawn()
//             .insert(Agent)
//             .insert(Transform::from_translation(Vec3::new(
//                 0.0,
//                 GRID_HEIGHT as f32 * CELL_SIZE / 2.0,
//                 0.0,
//             )))
//             .insert(GlobalTransform::default());
//     }
// }
