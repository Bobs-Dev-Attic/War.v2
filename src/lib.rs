use bevy::prelude::*;
use rand::{thread_rng, Rng};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const FIELD_SIZE: f32 = 90.0;
const UNIT_COUNT: usize = 28;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Side {
    Entente,
    Central,
}

#[derive(Component)]
struct Unit {
    side: Side,
    morale: f32,
    ammo: f32,
    health: f32,
}

#[derive(Component)]
struct Velocity(Vec3);

#[derive(Component)]
struct Projectile {
    ttl: Timer,
    damage: f32,
}

#[derive(Component)]
struct Particle {
    ttl: Timer,
    rise: f32,
}

#[derive(Resource)]
struct SelectedSide(Side);

#[derive(Resource)]
struct BattleClock(Timer);

impl Default for BattleClock {
    fn default() -> Self {
        Self(Timer::from_seconds(0.35, TimerMode::Repeating))
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    app().run();
}

pub fn app() -> App {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.54, 0.62, 0.68)))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 450.0,
        })
        .init_resource::<SelectedSide>()
        .init_resource::<BattleClock>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "War.v2 - WW1 Battlefield Simulator".into(),
                canvas: Some("#bevy".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_controls,
                command_input,
                unit_ai,
                integrate_physics,
                projectile_impacts,
                particle_lifecycle,
                update_hud,
            ),
        );
    app
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 48.0, 72.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 16_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-20.0, 50.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    let earth = materials.add(Color::srgb(0.28, 0.24, 0.16));
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(FIELD_SIZE, FIELD_SIZE)),
        material: earth.clone(),
        ..default()
    });

    let trench_mat = materials.add(Color::srgb(0.10, 0.075, 0.045));
    for z in [-14.0, 14.0] {
        for x in (-38..=38).step_by(8) {
            commands.spawn(PbrBundle {
                mesh: meshes.add(Cuboid::new(5.5, 0.7, 1.5)),
                material: trench_mat.clone(),
                transform: Transform::from_xyz(x as f32, 0.2, z),
                ..default()
            });
        }
    }
    spawn_clouds(&mut commands, &mut meshes, &mut materials);
    spawn_units(&mut commands, &mut meshes, &mut materials);

    commands.spawn(
        TextBundle::from_section(
            "War.v2",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 20.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(12.0),
            ..default()
        }),
    );
}

fn spawn_units(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let entente = materials.add(Color::srgb(0.12, 0.20, 0.55));
    let central = materials.add(Color::srgb(0.38, 0.34, 0.26));
    let mut rng = thread_rng();
    for i in 0..UNIT_COUNT {
        for side in [Side::Entente, Side::Central] {
            let z = if side == Side::Entente { -26.0 } else { 26.0 } + rng.gen_range(-4.0..4.0);
            let x = (i as f32 % 14.0) * 5.5 - 36.0 + rng.gen_range(-1.4..1.4);
            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Capsule3d::new(0.55, 1.0)),
                    material: if side == Side::Entente {
                        entente.clone()
                    } else {
                        central.clone()
                    },
                    transform: Transform::from_xyz(x, 1.0, z),
                    ..default()
                },
                Unit {
                    side,
                    morale: 1.0,
                    ammo: 30.0,
                    health: 100.0,
                },
                Velocity(Vec3::ZERO),
            ));
        }
    }
}

fn spawn_clouds(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cloud = materials.add(StandardMaterial {
        base_color: Color::srgba(0.86, 0.86, 0.82, 0.46),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 1.0,
        ..default()
    });
    for i in 0..18 {
        commands.spawn(PbrBundle {
            mesh: meshes.add(Sphere::new(3.5 + (i % 4) as f32)),
            material: cloud.clone(),
            transform: Transform::from_xyz(
                -42.0 + i as f32 * 5.0,
                22.0 + (i % 3) as f32,
                -30.0 + (i % 6) as f32 * 10.0,
            ),
            ..default()
        });
    }
}

fn command_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedSide>,
    mut q: Query<(&Unit, &mut Velocity)>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        selected.0 = Side::Entente;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        selected.0 = Side::Central;
    }
    let impulse = if keys.pressed(KeyCode::ArrowUp) {
        Vec3::new(0.0, 0.0, 8.0)
    } else if keys.pressed(KeyCode::ArrowDown) {
        Vec3::new(0.0, 0.0, -8.0)
    } else if keys.pressed(KeyCode::ArrowLeft) {
        Vec3::new(-8.0, 0.0, 0.0)
    } else if keys.pressed(KeyCode::ArrowRight) {
        Vec3::new(8.0, 0.0, 0.0)
    } else {
        Vec3::ZERO
    };
    for (unit, mut velocity) in &mut q {
        if unit.side == selected.0 {
            velocity.0 += impulse * 0.016;
        }
    }
}

fn unit_ai(
    time: Res<Time>,
    mut clock: ResMut<BattleClock>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &Transform, &mut Unit, &mut Velocity)>,
) {
    clock.0.tick(time.delta());
    let positions: Vec<_> = q
        .iter()
        .map(|(e, t, u, _)| (e, t.translation, u.side))
        .collect();
    let bullet_mat = materials.add(Color::srgb(0.95, 0.75, 0.22));
    for (_, transform, mut unit, mut velocity) in &mut q {
        if unit.health <= 0.0 {
            continue;
        }
        if let Some((_, target, _)) =
            positions
                .iter()
                .filter(|(_, _, s)| *s != unit.side)
                .min_by(|a, b| {
                    transform
                        .translation
                        .distance_squared(a.1)
                        .total_cmp(&transform.translation.distance_squared(b.1))
                })
        {
            let dir = (*target - transform.translation).normalize_or_zero();
            velocity.0 += dir * unit.morale * time.delta_seconds() * 1.6;
            if clock.0.just_finished()
                && unit.ammo > 0.0
                && transform.translation.distance(*target) < 38.0
            {
                unit.ammo -= 1.0;
                commands.spawn((
                    PbrBundle {
                        mesh: meshes.add(Sphere::new(0.12)),
                        material: bullet_mat.clone(),
                        transform: Transform::from_translation(transform.translation + Vec3::Y),
                        ..default()
                    },
                    Projectile {
                        ttl: Timer::from_seconds(2.0, TimerMode::Once),
                        damage: 18.0,
                    },
                    Velocity(dir * 42.0 + Vec3::Y * 2.0),
                ));
            }
        }
    }
}

fn integrate_physics(
    time: Res<Time>,
    mut q: Query<(&mut Transform, &mut Velocity), Without<Camera>>,
) {
    for (mut transform, mut velocity) in &mut q {
        velocity.0 += Vec3::new(0.0, -9.8, 0.0) * time.delta_seconds();
        transform.translation += velocity.0 * time.delta_seconds();
        transform.translation.y = transform.translation.y.max(0.6);
        velocity.0 *= 0.94;
        transform.translation.x = transform.translation.x.clamp(-44.0, 44.0);
        transform.translation.z = transform.translation.z.clamp(-44.0, 44.0);
    }
}

fn projectile_impacts(
    mut commands: Commands,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut projectiles: Query<(Entity, &Transform, &mut Projectile)>,
    mut units: Query<(&Transform, &mut Unit)>,
) {
    for (entity, transform, mut projectile) in &mut projectiles {
        projectile.ttl.tick(time.delta());
        let mut hit = projectile.ttl.finished() || transform.translation.y <= 0.7;
        for (unit_t, mut unit) in &mut units {
            if transform.translation.distance(unit_t.translation) < 1.2 {
                unit.health -= projectile.damage;
                unit.morale *= 0.92;
                hit = true;
            }
        }
        if hit {
            spawn_smoke(
                &mut commands,
                &mut meshes,
                &mut materials,
                transform.translation,
            );
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_smoke(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    origin: Vec3,
) {
    let smoke = materials.add(StandardMaterial {
        base_color: Color::srgba(0.22, 0.20, 0.18, 0.55),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    for i in 0..7 {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Sphere::new(0.7 + i as f32 * 0.08)),
                material: smoke.clone(),
                transform: Transform::from_translation(
                    origin + Vec3::new(i as f32 * 0.18 - 0.6, 0.4, 0.0),
                ),
                ..default()
            },
            Particle {
                ttl: Timer::from_seconds(1.8, TimerMode::Once),
                rise: 1.5 + i as f32 * 0.2,
            },
        ));
    }
}

fn particle_lifecycle(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &mut Particle)>,
) {
    for (e, mut t, mut p) in &mut q {
        p.ttl.tick(time.delta());
        t.translation.y += p.rise * time.delta_seconds();
        t.scale *= 1.0 + time.delta_seconds() * 0.35;
        if p.ttl.finished() {
            commands.entity(e).despawn();
        }
    }
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut q: Query<&mut Transform, With<Camera>>,
) {
    let mut t = q.single_mut();
    let speed = 28.0 * time.delta_seconds();
    if keys.pressed(KeyCode::KeyW) {
        t.translation.z -= speed;
    }
    if keys.pressed(KeyCode::KeyS) {
        t.translation.z += speed;
    }
    if keys.pressed(KeyCode::KeyA) {
        t.translation.x -= speed;
    }
    if keys.pressed(KeyCode::KeyD) {
        t.translation.x += speed;
    }
}

fn update_hud() {}

impl Default for SelectedSide {
    fn default() -> Self {
        Self(Side::Entente)
    }
}
