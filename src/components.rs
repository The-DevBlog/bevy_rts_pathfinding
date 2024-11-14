use bevy::prelude::*;

use crate::*;

#[derive(Component)]
pub struct MapBase;

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
pub struct Destination;

#[derive(Component)]
pub struct Selected;

#[derive(Component, Clone)]
pub struct FlowField {
    pub cells: Vec<Vec<Cell>>,
    pub destination: (usize, usize),
    pub entities: Vec<Entity>,
}

impl FlowField {
    pub fn new(rows: usize, columns: usize, target_row: usize, target_column: usize) -> Self {
        let mut grid = vec![
            vec![
                Cell {
                    position: Vec3::ZERO,
                    cost: f32::INFINITY,
                    flow_vector: Vec3::ZERO,
                    occupied: false,
                };
                rows
            ];
            columns
        ];

        // Calculate the offset to center the grid at (0, 0, 0)
        let grid_width = rows as f32 * CELL_SIZE;
        let grid_depth = columns as f32 * CELL_SIZE;
        let half_grid_width = grid_width / 2.0;
        let half_grid_depth = grid_depth / 2.0;

        for x in 0..rows {
            for z in 0..columns {
                let world_x = x as f32 * CELL_SIZE - half_grid_width + CELL_SIZE / 2.0;
                let world_z = z as f32 * CELL_SIZE - half_grid_depth + CELL_SIZE / 2.0;

                grid[x][z].position = Vec3::new(world_x, 0.0, world_z);
            }
        }

        grid[target_row][target_column].cost = 0.0;

        FlowField {
            cells: grid,
            destination: (target_row, target_column),
            entities: Vec::new(),
        }
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Cell {
    pub position: Vec3,
    pub cost: f32,
    pub flow_vector: Vec3,
    pub occupied: bool,
}
