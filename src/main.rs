//! War.v2 — a small grid-based tank skirmish, written in Bevy and compiled to
//! WebAssembly for the browser.
//!
//! Core rule: **no two vehicles may occupy the same cell.** The player tank
//! (green) moves one cell at a time with the arrow keys / WASD; the enemy
//! tanks wander on their own. Every move — player or AI — is rejected if the
//! target cell is off the board or already occupied by another vehicle.
//!
//! It exercises the real Bevy render loop, sprites, parent/child transforms,
//! keyboard input, timers and per-frame movement, all running as static wasm.

use bevy::color::palettes::css;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;

// ---- Board geometry -------------------------------------------------------

const GRID_W: i32 = 12;
const GRID_H: i32 = 8;
const CELL: f32 = 64.0;
const TANK_SIZE: f32 = 46.0;

/// Convert a logical grid cell to a world-space position (board centered on 0).
fn cell_to_world(cell: IVec2) -> Vec3 {
    let x = (cell.x as f32 - (GRID_W as f32 - 1.0) * 0.5) * CELL;
    let y = (cell.y as f32 - (GRID_H as f32 - 1.0) * 0.5) * CELL;
    Vec3::new(x, y, 1.0)
}

fn in_bounds(cell: IVec2) -> bool {
    cell.x >= 0 && cell.x < GRID_W && cell.y >= 0 && cell.y < GRID_H
}

// ---- Components & resources ----------------------------------------------

/// Any tank / vehicle on the board. `cell` is its logical position; the visual
/// transform smoothly follows it. `facing` points the barrel.
#[derive(Component)]
struct Vehicle {
    cell: IVec2,
    facing: IVec2,
}

/// Marks the human-controlled tank.
#[derive(Component)]
struct Player;

/// Marks an AI tank and holds its "think" timer.
#[derive(Component)]
struct Enemy {
    timer: Timer,
}

/// Tiny deterministic RNG so we don't need an external crate (and stay
/// reproducible for `Date.now`-free wasm builds).
#[derive(Resource)]
struct Rng(u32);

impl Rng {
    fn next_u32(&mut self) -> u32 {
        // xorshift32
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// One of the four cardinal directions.
    fn dir(&mut self) -> IVec2 {
        match self.next_u32() % 4 {
            0 => IVec2::new(1, 0),
            1 => IVec2::new(-1, 0),
            2 => IVec2::new(0, 1),
            _ => IVec2::new(0, -1),
        }
    }
}

// ---- App ------------------------------------------------------------------

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "War.v2 — tank skirmish (Bevy + WebAssembly)".into(),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.05, 0.07, 0.12)))
        .insert_resource(Rng(0x9e37_79b9))
        .add_systems(Startup, setup)
        // Player and AI movement are chained so occupancy stays consistent
        // within a frame (no two vehicles claim the same cell at once).
        .add_systems(Update, (player_move, enemy_move, follow_cell).chain())
        .run();
}

fn setup(mut commands: Commands) {
    // A 2D camera with tonemapping off — the trimmed render feature set does
    // not ship the tonemapping LUT, and leaving it on hides all sprites.
    commands.spawn((Camera2d, Tonemapping::None, Msaa::Off));

    draw_board(&mut commands);

    // Titles / instructions in world space above the board.
    let board_top = (GRID_H as f32 * CELL) * 0.5;
    commands.spawn((
        Text2d::new("War.v2  -  tank skirmish"),
        TextFont {
            font_size: FontSize::Px(28.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, board_top + 44.0, 10.0),
    ));
    commands.spawn((
        Text2d::new("arrow keys / WASD to move   -   no two vehicles share a cell"),
        TextFont {
            font_size: FontSize::Px(17.0),
            ..default()
        },
        TextColor(Color::srgb(0.62, 0.72, 0.88)),
        Transform::from_xyz(0.0, board_top + 18.0, 10.0),
    ));

    // The player tank.
    spawn_tank(&mut commands, IVec2::new(1, 1), css::LIMEGREEN, true);

    // A handful of enemy tanks at fixed starting cells.
    let enemies = [
        IVec2::new(10, 6),
        IVec2::new(10, 1),
        IVec2::new(1, 6),
        IVec2::new(6, 3),
        IVec2::new(8, 4),
    ];
    for (i, cell) in enemies.iter().enumerate() {
        let color = if i % 2 == 0 { css::CRIMSON } else { css::ORANGE };
        spawn_tank(&mut commands, *cell, color, false);
    }
}

/// Draw the board background and grid lines out of simple sprites.
fn draw_board(commands: &mut Commands) {
    let board_w = GRID_W as f32 * CELL;
    let board_h = GRID_H as f32 * CELL;

    // Felt-green playfield.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.12, 0.18, 0.15),
            custom_size: Some(Vec2::new(board_w, board_h)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let line = Color::srgba(1.0, 1.0, 1.0, 0.06);
    // Vertical grid lines.
    for i in 0..=GRID_W {
        let x = (i as f32 - GRID_W as f32 * 0.5) * CELL;
        commands.spawn((
            Sprite {
                color: line,
                custom_size: Some(Vec2::new(2.0, board_h)),
                ..default()
            },
            Transform::from_xyz(x, 0.0, 0.5),
        ));
    }
    // Horizontal grid lines.
    for j in 0..=GRID_H {
        let y = (j as f32 - GRID_H as f32 * 0.5) * CELL;
        commands.spawn((
            Sprite {
                color: line,
                custom_size: Some(Vec2::new(board_w, 2.0)),
                ..default()
            },
            Transform::from_xyz(0.0, y, 0.5),
        ));
    }
}

/// Spawn a tank: a body sprite with a barrel child so it visibly aims.
fn spawn_tank(commands: &mut Commands, cell: IVec2, color: Srgba, is_player: bool) {
    let facing = IVec2::new(0, 1);
    let mut tank = commands.spawn((
        Sprite {
            color: color.into(),
            custom_size: Some(Vec2::splat(TANK_SIZE)),
            ..default()
        },
        Transform::from_translation(cell_to_world(cell)),
        Vehicle { cell, facing },
    ));

    // Barrel: a thin light rectangle extending "up" in local space, so
    // rotating the tank aims it along `facing`.
    tank.with_children(|parent| {
        parent.spawn((
            Sprite {
                color: Color::srgb(0.9, 0.9, 0.9),
                custom_size: Some(Vec2::new(7.0, TANK_SIZE * 0.75)),
                ..default()
            },
            Transform::from_xyz(0.0, TANK_SIZE * 0.45, 0.1),
        ));
    });

    if is_player {
        tank.insert(Player);
    } else {
        tank.insert(Enemy {
            timer: Timer::from_seconds(0.6, TimerMode::Repeating),
        });
    }
}

// ---- Movement -------------------------------------------------------------

/// Return true if `cell` is on the board and not held by any vehicle other
/// than `mover`.
fn cell_free(cell: IVec2, mover: Entity, occ: &[(Entity, IVec2)]) -> bool {
    in_bounds(cell) && !occ.iter().any(|(e, c)| *e != mover && *c == cell)
}

/// Player tank: one cell per key press; blocked by edges and other vehicles.
///
/// The read query excludes the player (`Without<Player>`) so it stays disjoint
/// from the mutable player query; the player's own cell is irrelevant to the
/// occupancy test anyway (`cell_free` ignores the mover).
fn player_move(
    keys: Res<ButtonInput<KeyCode>>,
    others: Query<(Entity, &Vehicle), Without<Player>>,
    mut player: Query<(Entity, &mut Vehicle), With<Player>>,
) {
    let dir = if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) {
        IVec2::new(0, 1)
    } else if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) {
        IVec2::new(0, -1)
    } else if keys.just_pressed(KeyCode::ArrowLeft) || keys.just_pressed(KeyCode::KeyA) {
        IVec2::new(-1, 0)
    } else if keys.just_pressed(KeyCode::ArrowRight) || keys.just_pressed(KeyCode::KeyD) {
        IVec2::new(1, 0)
    } else {
        return;
    };

    let occ: Vec<(Entity, IVec2)> = others.iter().map(|(e, v)| (e, v.cell)).collect();
    if let Ok((entity, mut vehicle)) = player.single_mut() {
        vehicle.facing = dir; // always turn to face, even if blocked
        let target = vehicle.cell + dir;
        if cell_free(target, entity, &occ) {
            vehicle.cell = target;
        }
    }
}

/// Enemy tanks: each ticks a timer, then tries a random cardinal step, obeying
/// the same occupancy rule. Occupancy is updated as we go so two enemies never
/// step onto the same cell in one frame.
///
/// A `ParamSet` lets us read every vehicle's cell (including the player) and
/// then mutate the enemies without the two queries conflicting.
fn enemy_move(
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut set: ParamSet<(
        Query<(Entity, &Vehicle)>,
        Query<(Entity, &mut Vehicle, &mut Enemy)>,
    )>,
) {
    let mut occ: Vec<(Entity, IVec2)> = set.p0().iter().map(|(e, v)| (e, v.cell)).collect();

    for (entity, mut vehicle, mut enemy) in &mut set.p1() {
        if !enemy.timer.tick(time.delta()).just_finished() {
            continue;
        }
        let dir = rng.dir();
        vehicle.facing = dir;
        let target = vehicle.cell + dir;
        if cell_free(target, entity, &occ) {
            // Update both the component and our local occupancy snapshot.
            if let Some(slot) = occ.iter_mut().find(|(e, _)| *e == entity) {
                slot.1 = target;
            }
            vehicle.cell = target;
        }
    }
}

/// Smoothly move each tank's transform toward its logical cell and aim it.
fn follow_cell(time: Res<Time>, mut vehicles: Query<(&Vehicle, &mut Transform)>) {
    let dt = time.delta_secs();
    for (vehicle, mut transform) in &mut vehicles {
        let target = cell_to_world(vehicle.cell);
        // Exponential smoothing toward the target cell.
        let t = 1.0 - (-12.0 * dt).exp();
        transform.translation = transform.translation.lerp(target, t);

        // Aim: local +Y should point along `facing`.
        let angle = (vehicle.facing.y as f32).atan2(vehicle.facing.x as f32)
            - std::f32::consts::FRAC_PI_2;
        let target_rot = Quat::from_rotation_z(angle);
        transform.rotation = transform.rotation.slerp(target_rot, t);
    }
}
