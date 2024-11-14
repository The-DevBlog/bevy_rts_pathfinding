use crate::*;

const ARROW_LENGTH: f32 = CELL_SIZE * 0.75 / 2.0;

pub struct BevyRtsPathFindingDebugPlugin;

impl Plugin for BevyRtsPathFindingDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_flowfield, draw_grid));
    }
}

fn draw_flowfield(
    flowfield_q: Query<&FlowField>,
    selected_q: Query<Entity, With<Selected>>,
    grid: Res<Grid>,
    mut gizmos: Gizmos,
) {
    if selected_q.is_empty() {
        return;
    }

    let mut selected_entity_ids = Vec::new();
    for selected_entity in selected_q.iter() {
        selected_entity_ids.push(selected_entity);
    }

    for flowfield in flowfield_q.iter() {
        if !selected_entity_ids
            .iter()
            .any(|item| flowfield.entities.contains(item))
        {
            continue;
        }

        // Drag circle for target cell
        let target_cell = &flowfield.cells[flowfield.destination.0][flowfield.destination.1];
        let dir = Dir3::from_xyz(0.0, 1.0, 0.0).unwrap();
        gizmos.circle(target_cell.position, dir, ARROW_LENGTH / 1.5, YELLOW);

        for x in 0..grid.rows {
            for z in 0..grid.columns {
                let cell = &flowfield.cells[x][z];

                if target_cell == cell {
                    continue;
                }

                if cell.occupied || cell.flow_vector == Vec3::ZERO {
                    // Draw an 'X' for each occupied cell
                    let top_left = cell.position + Vec3::new(-ARROW_LENGTH, 0.0, -ARROW_LENGTH);
                    let top_right = cell.position + Vec3::new(ARROW_LENGTH, 0.0, -ARROW_LENGTH);
                    let bottom_left = cell.position + Vec3::new(-ARROW_LENGTH, 0.0, ARROW_LENGTH);
                    let bottom_right = cell.position + Vec3::new(ARROW_LENGTH, 0.0, ARROW_LENGTH);

                    gizmos.line(top_left, bottom_right, RED);
                    gizmos.line(top_right, bottom_left, RED);
                    continue;
                }

                let flow_direction = cell.flow_vector.normalize();

                let start = cell.position - flow_direction * ARROW_LENGTH;
                let end = cell.position + flow_direction * ARROW_LENGTH;

                gizmos.arrow(start, end, COLOR_ARROWS);
            }
        }
    }
}

fn draw_grid(mut gizmos: Gizmos, grid: Res<Grid>) {
    gizmos.grid(
        Vec3::ZERO,
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        UVec2::new(grid.rows as u32, grid.columns as u32),
        Vec2::new(CELL_SIZE, CELL_SIZE),
        COLOR_GRID,
    );
}
