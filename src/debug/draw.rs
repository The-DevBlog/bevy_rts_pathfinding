use super::components::*;
use super::events::*;
use super::resources::*;
use crate::*;

use cell::Cell;
use debug::COLOR_GRID;
use events::UpdateCostEv;
use grid::Grid;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

const BASE_SCALE: f32 = 0.25;

pub struct DrawPlugin;

impl Plugin for DrawPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                draw_grid,
                detect_debug_change,
                update_cell_cost.after(grid::update_costs),
            ),
        )
        .add_observer(set_active_dbg_flowfield)
        .add_observer(draw_costfield)
        .add_observer(draw_flowfield)
        .add_observer(draw_integration_field)
        .add_observer(draw_index);
    }
}

fn set_active_dbg_flowfield(
    trigger: Trigger<SetActiveFlowfieldEv>,
    mut cmds: Commands,
    mut active_dbg_flowfield: ResMut<ActiveDebugFlowfield>,
) {
    if let Some(new_flowfield) = &trigger.event().0 {
        if let Some(current_flowfield) = &active_dbg_flowfield.0 {
            // Skip if the grid is the same
            if current_flowfield.grid == new_flowfield.grid {
                return;
            }
        }
        // Set the new flowfield and trigger debug draw
        active_dbg_flowfield.0 = Some(new_flowfield.clone());
        cmds.trigger(DrawDebugEv);
    } else {
        // Deactivate if there’s no new flowfield
        if active_dbg_flowfield.0.is_some() {
            active_dbg_flowfield.0 = None;
            cmds.trigger(DrawDebugEv);
        }
    }
}

fn draw_grid(grid: Res<Grid>, mut gizmos: Gizmos, debug: Res<DebugOptions>) {
    if !debug.draw_grid {
        return;
    }

    gizmos.grid(
        Isometry3d::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        UVec2::new(grid.size.x as u32, grid.size.y as u32),
        Vec2::new(grid.cell_radius * 2.0, grid.cell_radius * 2.0),
        COLOR_GRID,
    );
}

// TODO: Cleanup this method
fn draw_flowfield(
    _trigger: Trigger<DrawDebugEv>,
    dbg: Res<DebugOptions>,
    grid: Res<Grid>,
    active_dbg_flowfield: Res<ActiveDebugFlowfield>,
    q_flowfield_arrow: Query<Entity, With<FlowFieldArrow>>,
    mut cmds: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Remove current arrows before rendering new ones
    for arrow_entity in &q_flowfield_arrow {
        cmds.entity(arrow_entity).despawn_recursive();
    }

    let Some(active_dbg_flowfield) = &active_dbg_flowfield.0 else {
        return;
    };

    let mut marker_scale = 0.7;
    if (dbg.draw_mode_1 == DrawMode::None || dbg.draw_mode_2 == DrawMode::None)
        || (dbg.draw_mode_1 == DrawMode::FlowField && dbg.draw_mode_2 == DrawMode::FlowField)
    {
        marker_scale = 1.0;
    }

    let offset = calculate_offset(active_dbg_flowfield.cell_diameter, dbg, DrawMode::FlowField);
    let Some(offset) = offset else {
        return;
    };

    println!("Drawing Flowfield");

    let arrow_length = grid.cell_diameter * 0.6 * marker_scale;
    let arrow_width = grid.cell_diameter * 0.1 * marker_scale;
    let arrow_clr = Color::WHITE;

    // Create the arrowhead mesh
    let half_arrow_size = arrow_length / 2.0;
    let d1 = half_arrow_size - grid.cell_diameter * 0.09;
    let d2 = arrow_width + grid.cell_diameter * 0.0125;
    let a = Vec2::new(half_arrow_size + grid.cell_diameter * 0.05, 0.0); // Tip of the arrowhead
    let b = Vec2::new(d1, d2);
    let c = Vec2::new(d1, -arrow_width - grid.cell_diameter * 0.0125);

    // Mesh for arrow
    let arrow_mesh = meshes.add(Plane3d::default().mesh().size(arrow_length, arrow_width));
    let arrow_head_mesh = meshes.add(Triangle2d::new(a, b, c));

    let material = materials.add(StandardMaterial {
        base_color: arrow_clr,
        ..default()
    });

    // println!("Drawing flowfield");
    for cell_row in &active_dbg_flowfield.grid {
        for cell in cell_row.iter() {
            let is_destination_cell = active_dbg_flowfield.destination_cell.idx == cell.idx;

            let rotation = match is_destination_cell {
                true => Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                false => Quat::from_rotation_y(cell.best_direction.to_angle()),
            };

            let mesh = match is_destination_cell {
                true => meshes.add(Circle::new(grid.cell_radius / 3.0 * marker_scale)),
                false => arrow_mesh.clone(),
            };

            let marker = (
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform {
                    translation: cell.world_pos + offset,
                    rotation,
                    ..default()
                },
                FlowFieldArrow,
                Name::new("Flowfield Marker Arrow"),
            );

            let arrow_head = (
                Mesh3d(arrow_head_mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                    ..default()
                },
                Name::new("Arrowhead"),
            );

            if cell.cost < u8::MAX {
                let mut draw = cmds.spawn(marker);

                if !is_destination_cell {
                    draw.with_children(|parent| {
                        parent.spawn(arrow_head);
                    });
                }
            } else {
                let cross = (
                    Transform::default(),
                    Mesh3d(mesh),
                    MeshMaterial3d(materials.add(StandardMaterial::from_color(RED))),
                    FlowFieldArrow,
                    Name::new("Flowfield Marker 'X'"),
                );

                let mut cross_1 = cross.clone();
                cross_1.0 = Transform {
                    translation: cell.world_pos + offset,
                    rotation: Quat::from_rotation_y(3.0 * FRAC_PI_4),
                    ..default()
                };

                let mut cross_2 = cross.clone();
                cross_2.0 = Transform {
                    translation: cell.world_pos + offset,
                    rotation: Quat::from_rotation_y(FRAC_PI_4),
                    ..default()
                };

                cmds.spawn(cross_1);
                cmds.spawn(cross_2);
            }
        }
        // println!();
    }
}

fn draw_integration_field(
    _trigger: Trigger<DrawDebugEv>,
    dbg: Res<DebugOptions>,
    digits: Res<Digits>,
    active_dbg_flowfield: Res<ActiveDebugFlowfield>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    q_cost: Query<Entity, With<BestCost>>,
    mut cmds: Commands,
) {
    // Remove current cost field before rendering new one
    for cost_entity in &q_cost {
        cmds.entity(cost_entity).despawn_recursive();
    }

    let Some(flowfield) = &active_dbg_flowfield.0 else {
        return;
    };

    let offset = calculate_offset(flowfield.cell_diameter, dbg, DrawMode::IntegrationField);
    let Some(offset) = offset else {
        return;
    };

    println!("Drawing Integration Field");

    let str = |cell: &Cell| format!("{}", cell.best_cost);
    draw(
        meshes,
        materials,
        &flowfield.grid,
        flowfield.cell_diameter,
        digits,
        BestCost,
        cmds,
        str,
        offset,
    );
}

fn draw_index(
    _trigger: Trigger<DrawDebugEv>,
    dbg: Res<DebugOptions>,
    active_dbg_flowfield: Res<ActiveDebugFlowfield>,
    digits: Res<Digits>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    q_idx: Query<Entity, With<Index>>,
    mut cmds: Commands,
) {
    // Remove current index entities before rendering new ones
    for idx_entity in &q_idx {
        cmds.entity(idx_entity).despawn_recursive();
    }

    let Some(flowfield) = &active_dbg_flowfield.0 else {
        return;
    };

    let offset = calculate_offset(flowfield.cell_diameter, dbg, DrawMode::Index);
    let Some(offset) = offset else {
        return;
    };

    println!("Drawing Index");

    let str = |cell: &Cell| format!("{}{}", cell.idx.y, cell.idx.x);
    draw(
        meshes,
        materials,
        &flowfield.grid,
        flowfield.cell_diameter,
        digits,
        Index,
        cmds,
        str,
        offset,
    );
}

fn draw_costfield(
    _trigger: Trigger<DrawDebugEv>,
    mut costmap: ResMut<CostMap>,
    dbg: Res<DebugOptions>,
    digits: Res<Digits>,
    mut meshes: ResMut<Assets<Mesh>>,
    grid: Res<Grid>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cmds: Commands,
    q_cost: Query<Entity, With<Cost>>,
) {
    // Remove current cost field before rendering new one
    for cost_entity in &q_cost {
        cmds.entity(cost_entity).despawn_recursive();
    }

    let base_offset = calculate_offset(grid.cell_diameter, dbg, DrawMode::CostField);
    let Some(base_offset) = base_offset else {
        return;
    };

    println!("Drawing Costfield");

    let base_digit_spacing = grid.cell_diameter * 0.275;
    let mesh = meshes.add(Rectangle::new(grid.cell_diameter, grid.cell_diameter));

    for cell_row in &grid.grid {
        for cell in cell_row.iter() {
            let digits_vec: Vec<u32> = cell
                .cost
                .to_string()
                .chars()
                .filter_map(|c| c.to_digit(10))
                .collect();

            let (scale, digit_spacing) = calculate_digit_spacing_and_scale(
                grid.cell_diameter,
                digits_vec.len(),
                base_digit_spacing,
            );

            let cost_entities = spawn_digit_entities(
                &mut cmds,
                &digits_vec,
                base_offset,
                scale,
                digit_spacing,
                cell.world_pos,
                &mut materials,
                &digits,
                mesh.clone(),
                Cost,
            );

            costmap.0.insert(cell.idx, cost_entities);
        }
    }
}

fn update_cell_cost(
    mut cmds: Commands,
    mut events: EventReader<UpdateCostEv>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut cost_map: ResMut<CostMap>,
    dbg: Res<DebugOptions>,
    digits: Res<Digits>,
    grid: Res<Grid>,
) {
    let base_digit_spacing = grid.cell_diameter * 0.275;
    let cell_diameter = grid.cell_diameter;

    let base_offset = calculate_offset(cell_diameter, dbg, DrawMode::CostField);
    let Some(base_offset) = base_offset else {
        return;
    };

    let mesh = meshes.add(Rectangle::new(cell_diameter, cell_diameter));

    for ev in events.read() {
        let cell = ev.cell;
        let digits_vec: Vec<u32> = cell
            .cost
            .to_string()
            .chars()
            .filter_map(|c| c.to_digit(10))
            .collect();

        let (scale, digit_spacing) =
            calculate_digit_spacing_and_scale(cell_diameter, digits_vec.len(), base_digit_spacing);

        let new_cost_entities = spawn_digit_entities(
            &mut cmds,
            &digits_vec,
            base_offset,
            scale,
            digit_spacing,
            cell.world_pos,
            &mut materials,
            &digits,
            mesh.clone(),
            Cost,
        );

        if let Some(previous_cost) = cost_map.0.remove(&cell.idx) {
            for entity in previous_cost {
                cmds.entity(entity).despawn();
            }
        }

        cost_map.0.insert(cell.idx, new_cost_entities);
    }
}

fn calculate_offset(
    cell_diameter: f32,
    dbg: Res<DebugOptions>,
    draw_mode: DrawMode,
) -> Option<Vec3> {
    let mode = if dbg.draw_mode_1 == draw_mode {
        Some(1)
    } else if dbg.draw_mode_2 == draw_mode {
        Some(2)
    } else {
        None
    };

    if mode.is_none() {
        return None; // nothing to draw
    }

    // Base offset when only one mode is active
    let mut offset = Vec3::new(0.0, 0.01, 0.0);
    if (dbg.draw_mode_1 == DrawMode::None || dbg.draw_mode_2 == DrawMode::None)
        || (dbg.draw_mode_1 == draw_mode && dbg.draw_mode_2 == draw_mode)
    {
        offset.z = 0.0;
    } else {
        match mode {
            Some(1) => offset.z = -cell_diameter * 0.25,
            Some(2) => offset.z = cell_diameter * 0.25,
            _ => (),
        };
    }

    return Some(offset);
}

fn draw<T: Component + Copy>(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cells: &Vec<Vec<Cell>>,
    cell_diameter: f32,
    digits: Res<Digits>,
    comp: T,
    mut cmds: Commands,
    get_str: impl Fn(&Cell) -> String,
    base_offset: Vec3,
) {
    let base_digit_spacing = cell_diameter * 0.275;

    let mesh = meshes.add(Rectangle::new(cell_diameter, cell_diameter));

    for cell_row in cells {
        for cell in cell_row.iter() {
            // Generate the string using the closure
            let value_str = get_str(cell);

            // Convert the string into individual digits
            let digits_vec: Vec<u32> = value_str.chars().filter_map(|c| c.to_digit(10)).collect();
            let (scale, digit_spacing) = calculate_digit_spacing_and_scale(
                cell_diameter,
                digits_vec.len(),
                base_digit_spacing,
            );

            spawn_digit_entities(
                &mut cmds,
                &digits_vec,
                base_offset,
                scale,
                digit_spacing,
                cell.world_pos,
                &mut materials,
                &digits,
                mesh.clone(),
                comp,
            );
        }
    }
}

fn calculate_digit_spacing_and_scale(
    cell_diameter: f32,
    digit_count: usize,
    base_digit_spacing: f32,
) -> (Vec3, f32) {
    let digit_width = cell_diameter * BASE_SCALE;
    let total_digit_width = digit_count as f32 * digit_width;
    let total_spacing_width = (digit_count as f32 - 1.0) * base_digit_spacing;
    let total_width = total_digit_width + total_spacing_width;

    if total_width > cell_diameter {
        let scale_factor = cell_diameter / total_width;
        (
            Vec3::splat(BASE_SCALE * scale_factor),
            base_digit_spacing * scale_factor,
        )
    } else {
        (Vec3::splat(BASE_SCALE), base_digit_spacing)
    }
}

fn spawn_digit_entities<T: Component + Copy>(
    cmds: &mut Commands,
    digits_vec: &[u32],
    base_offset: Vec3,
    scale: Vec3,
    digit_spacing: f32,
    cell_world_pos: Vec3,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    digits: &Res<Digits>,
    mesh: Handle<Mesh>,
    comp: T,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    let x_offset = -(digits_vec.len() as f32 - 1.0) * digit_spacing / 2.0;

    for (i, &digit) in digits_vec.iter().enumerate() {
        let mut offset = base_offset;
        offset.x += x_offset + i as f32 * digit_spacing;

        let material = materials.add(StandardMaterial {
            base_color_texture: Some(digits.0[digit as usize].clone()),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });

        let dig = (
            comp,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform {
                translation: cell_world_pos + offset,
                rotation: Quat::from_rotation_x(-FRAC_PI_2),
                scale,
            },
        );

        let entity = cmds.spawn(dig).id();
        entities.push(entity);
    }

    entities
}

fn detect_debug_change(mut cmds: Commands, debug: Res<DebugOptions>) {
    if debug.is_changed() {
        cmds.trigger(DrawDebugEv);
    }
}
