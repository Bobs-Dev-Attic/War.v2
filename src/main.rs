//! War.v2 — an isometric 3D WWI real-time strategy sim, written in Bevy and
//! compiled to WebAssembly.
//!
//! This is the v0.3.0 foundation: an orthographic isometric camera over a
//! low-poly battlefield, with two armies of low-poly tanks — the British
//! (khaki) and the Central Powers / Ottoman-German (field-grey) — advancing on
//! no-man's-land. Units separate so they never overlap, giving the massed
//! formations a jostling, chaotic feel ahead of real rigid-body physics
//! (planned for v0.4.0).
//!
//! Controls: WASD / arrow keys pan the camera across the battlefield.

use bevy::color::palettes::css;
use bevy::prelude::*;
use bevy::camera::ScalingMode;

// ---- Battlefield geometry -------------------------------------------------

const FIELD_X: f32 = 34.0; // half-width  (east/west)
const FIELD_Z: f32 = 46.0; // half-depth  (north/south)
const UNIT_RADIUS: f32 = 1.6; // separation radius (no overlap)
const ROWS: i32 = 4;
const COLS: i32 = 8;

/// The two belligerents. The player will command one of these in a later
/// version; for now both advance under their own steam.
#[derive(Clone, Copy, PartialEq)]
enum Faction {
    /// British Empire — advancing from the south (−Z) toward the line.
    British,
    /// Central Powers (Ottoman / German) — advancing from the north (+Z).
    Central,
}

impl Faction {
    fn color(self) -> Srgba {
        match self {
            Faction::British => Srgba::new(0.62, 0.56, 0.36, 1.0), // khaki
            Faction::Central => Srgba::new(0.36, 0.40, 0.34, 1.0), // field-grey
        }
    }
    /// The direction this faction advances (world +Z or −Z).
    fn advance(self) -> Vec3 {
        match self {
            Faction::British => Vec3::Z,
            Faction::Central => Vec3::NEG_Z,
        }
    }
}

/// A vehicle on the field. `speed` is its forward march rate; separation from
/// neighbours is applied on top so vehicles never occupy the same space.
#[derive(Component)]
struct Unit {
    faction: Faction,
    speed: f32,
}

/// Camera focus point on the ground; the iso camera is a fixed offset from it.
#[derive(Resource)]
struct CameraFocus(Vec3);

/// Tiny deterministic RNG (no external crate, reproducible for wasm).
#[derive(Resource)]
struct Rng(u32);
impl Rng {
    fn f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / (1u32 << 24) as f32 // 0..1
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
}

// Isometric camera: a fixed offset direction from the focus, orthographic.
const ISO_DIR: Vec3 = Vec3::new(1.0, 1.15, 1.0);
const CAM_DIST: f32 = 90.0;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "War.v2 — WWI (Bevy + WebAssembly)".into(),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.63, 0.67, 0.72))) // hazy sky
        .insert_resource(CameraFocus(Vec3::ZERO))
        .insert_resource(Rng(0x1234_5678))
        .add_systems(Startup, (setup_world, setup_ui))
        .add_systems(Update, (advance_units, pan_camera).chain())
        .run();
}

// ---- World setup ----------------------------------------------------------

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<Rng>,
) {
    // Isometric orthographic camera, with a camera-attached ambient light.
    commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 60.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(ISO_DIR.normalize() * CAM_DIST).looking_at(Vec3::ZERO, Vec3::Y),
        AmbientLight {
            color: Color::srgb(0.85, 0.86, 0.90),
            brightness: 320.0,
            ..default()
        },
    ));

    // Sun: a directional light with shadows for the low-poly look.
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground: a big matte mud slab.
    let ground = meshes.add(Cuboid::new(FIELD_X * 2.0 + 8.0, 1.0, FIELD_Z * 2.0 + 8.0));
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.26, 0.19),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(ground),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));

    // No-man's-land: a darker churned strip through the middle.
    let strip = meshes.add(Cuboid::new(FIELD_X * 2.0, 0.06, 10.0));
    let strip_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.17, 0.15, 0.12),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(strip),
        MeshMaterial3d(strip_mat),
        Transform::from_xyz(0.0, 0.03, 0.0),
    ));

    // Scatter low-poly craters / rubble for texture.
    let rubble_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.18, 0.14),
        perceptual_roughness: 1.0,
        ..default()
    });
    for _ in 0..60 {
        let s = rng.range(0.6, 2.2);
        let x = rng.range(-FIELD_X, FIELD_X);
        let z = rng.range(-FIELD_Z, FIELD_Z);
        let mesh = meshes.add(Cuboid::new(s, rng.range(0.15, 0.5), s));
        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(rubble_mat.clone()),
            Transform::from_xyz(x, 0.05, z)
                .with_rotation(Quat::from_rotation_y(rng.range(0.0, 6.28))),
        ));
    }

    // Deploy the two armies in loose formation on opposite ends.
    spawn_army(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut rng,
        Faction::British,
        -FIELD_Z + 6.0,
    );
    spawn_army(
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut rng,
        Faction::Central,
        FIELD_Z - 6.0,
    );
}

/// Spawn one faction's formation at `base_z`, facing the centre line.
fn spawn_army(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    rng: &mut Rng,
    faction: Faction,
    base_z: f32,
) {
    // Shared low-poly tank meshes (built once, instanced per unit).
    let hull = meshes.add(Cuboid::new(2.2, 0.9, 3.2));
    let turret = meshes.add(Cuboid::new(1.4, 0.7, 1.4));
    let barrel = meshes.add(Cuboid::new(0.22, 0.22, 2.0));
    let body_mat = materials.add(StandardMaterial {
        base_color: faction.color().into(),
        perceptual_roughness: 0.95,
        ..default()
    });
    let metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.10, 0.10),
        perceptual_roughness: 0.6,
        ..default()
    });

    let advance = faction.advance();
    let facing = Quat::from_rotation_arc(Vec3::Z, advance);

    for row in 0..ROWS {
        for col in 0..COLS {
            let x = (col as f32 - (COLS as f32 - 1.0) * 0.5) * 5.0 + rng.range(-0.8, 0.8);
            let z = base_z - advance.z * (row as f32 * 4.5) + rng.range(-0.8, 0.8);

            commands
                .spawn((
                    Transform::from_xyz(x, 0.45, z).with_rotation(facing),
                    Visibility::default(),
                    Unit {
                        faction,
                        speed: rng.range(1.4, 2.4),
                    },
                ))
                .with_children(|t| {
                    t.spawn((
                        Mesh3d(hull.clone()),
                        MeshMaterial3d(body_mat.clone()),
                        Transform::default(),
                    ));
                    t.spawn((
                        Mesh3d(turret.clone()),
                        MeshMaterial3d(body_mat.clone()),
                        Transform::from_xyz(0.0, 0.7, -0.2),
                    ));
                    t.spawn((
                        Mesh3d(barrel.clone()),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_xyz(0.0, 0.75, 1.0),
                    ));
                });
        }
    }
}

// ---- UI overlay -----------------------------------------------------------

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(14.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|p| {
            p.spawn((
                Text::new("War.v2  -  WWI Western Front"),
                TextFont {
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgb(0.12, 0.12, 0.12)),
            ));
            p.spawn((
                Text::new("British (khaki)  vs  Central Powers (field-grey)"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.20, 0.20, 0.20)),
            ));
            p.spawn((
                Text::new("WASD / arrows: pan the battlefield"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.25, 0.25, 0.25)),
            ));
        });

    // Version badge, bottom-right, wired to the crate version.
    commands.spawn((
        Text::new(concat!("v", env!("CARGO_PKG_VERSION"))),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.15, 0.15, 0.15)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(12.0),
            ..default()
        },
    ));
}

// ---- Simulation -----------------------------------------------------------

/// March every unit forward, then push overlapping units apart so no two
/// vehicles occupy the same space. Units halt around no-man's-land.
fn advance_units(time: Res<Time>, mut units: Query<(Entity, &mut Transform, &Unit)>) {
    let dt = time.delta_secs();

    // Snapshot positions so separation reads a consistent frame.
    let snapshot: Vec<(Entity, Vec3)> = units
        .iter()
        .map(|(e, t, _)| (e, t.translation))
        .collect();

    for (entity, mut transform, unit) in &mut units {
        let mut pos = transform.translation;
        let advance = unit.faction.advance();

        // March forward until the unit nears the centre line, then hold.
        let dist_to_line = pos.z.abs();
        if dist_to_line > 6.0 {
            pos += advance * unit.speed * dt;
        }

        // Separation: push away from any neighbour within UNIT_RADIUS*2.
        let mut push = Vec3::ZERO;
        for (other, opos) in &snapshot {
            if *other == entity {
                continue;
            }
            let mut d = pos - *opos;
            d.y = 0.0;
            let dist = d.length();
            let min_d = UNIT_RADIUS * 2.0;
            if dist > 0.0001 && dist < min_d {
                push += d.normalize() * (min_d - dist);
            }
        }
        pos += push * 0.5;

        // Keep on the field.
        pos.x = pos.x.clamp(-FIELD_X, FIELD_X);
        pos.z = pos.z.clamp(-FIELD_Z, FIELD_Z);
        pos.y = 0.45;

        // Face the direction of travel (advance + jostle).
        let mut heading = advance + push.normalize_or_zero() * 0.3;
        heading.y = 0.0;
        if heading.length() > 0.01 {
            let yaw = Quat::from_rotation_arc(Vec3::Z, heading.normalize());
            transform.rotation = transform.rotation.slerp(yaw, 1.0 - (-6.0 * dt).exp());
        }
        transform.translation = pos;
    }
}

/// Pan the isometric camera across the field with WASD / arrow keys.
fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut focus: ResMut<CameraFocus>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        dir += Vec3::new(-1.0, 0.0, -1.0);
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        dir += Vec3::new(1.0, 0.0, 1.0);
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        dir += Vec3::new(-1.0, 0.0, 1.0);
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        dir += Vec3::new(1.0, 0.0, -1.0);
    }

    if dir != Vec3::ZERO {
        focus.0 += dir.normalize() * 24.0 * time.delta_secs();
        focus.0.x = focus.0.x.clamp(-FIELD_X, FIELD_X);
        focus.0.z = focus.0.z.clamp(-FIELD_Z, FIELD_Z);
    }

    if let Ok(mut transform) = cam.single_mut() {
        let eye = focus.0 + ISO_DIR.normalize() * CAM_DIST;
        *transform = Transform::from_translation(eye).looking_at(focus.0, Vec3::Y);
    }
}

// Keep a reference to css so the palette import is always available for tuning.
#[allow(dead_code)]
const _PALETTE: Srgba = css::KHAKI;
