//! War.v2 — an isometric 3D WWI real-time strategy sim (Bevy → WebAssembly).
//!
//! v0.4.0: trench warfare. Each side digs in behind a sandbagged trench line,
//! garrisoned by a roster of low-poly unit types — infantry, snipers, machine
//! gunners, artillery, runners and scouts. This is the deployed battlefield;
//! interactive setup (v0.5.0) and real-time commands (v0.6.0) build on it.
//!
//! Controls: WASD / arrow keys pan the camera across the front.

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::camera::ScalingMode;
use bevy::color::LinearRgba;
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::winit::{UpdateMode, WinitSettings};
use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

// ---- Setup config (chosen on the HTML setup page) -------------------------

/// WWI theatre — sets the terrain palette and crater density.
#[derive(Clone, Copy)]
enum Location {
    Somme,
    Ypres,
    Gallipoli,
    Sinai,
    Verdun,
}

/// Weather — sets sky, lighting and fog.
#[derive(Clone, Copy)]
enum Weather {
    Clear,
    Overcast,
    Rain,
    Fog,
    Snow,
}

#[derive(Resource, Clone, Copy)]
struct Setup {
    location: Location,
    weather: Weather,
}

impl Location {
    fn parse(s: &str) -> Self {
        match s {
            "ypres" => Location::Ypres,
            "gallipoli" => Location::Gallipoli,
            "sinai" => Location::Sinai,
            "verdun" => Location::Verdun,
            _ => Location::Somme,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Location::Somme => "The Somme",
            Location::Ypres => "Ypres / Flanders",
            Location::Gallipoli => "Gallipoli",
            Location::Sinai => "Sinai & Palestine",
            Location::Verdun => "Verdun",
        }
    }
    /// (ground color, no-man's-land color, crater color, crater count)
    fn palette(self) -> (Color, Color, Color, u32) {
        match self {
            Location::Somme => (
                Color::srgb(0.30, 0.27, 0.19),
                Color::srgb(0.18, 0.16, 0.12),
                Color::srgb(0.21, 0.19, 0.14),
                80,
            ),
            Location::Ypres => (
                Color::srgb(0.24, 0.26, 0.18),
                Color::srgb(0.15, 0.16, 0.12),
                Color::srgb(0.17, 0.18, 0.13),
                95,
            ),
            Location::Gallipoli => (
                Color::srgb(0.50, 0.42, 0.26),
                Color::srgb(0.40, 0.33, 0.20),
                Color::srgb(0.44, 0.36, 0.22),
                45,
            ),
            Location::Sinai => (
                Color::srgb(0.66, 0.56, 0.36),
                Color::srgb(0.58, 0.48, 0.30),
                Color::srgb(0.60, 0.50, 0.32),
                25,
            ),
            Location::Verdun => (
                Color::srgb(0.26, 0.25, 0.18),
                Color::srgb(0.16, 0.15, 0.11),
                Color::srgb(0.19, 0.18, 0.13),
                90,
            ),
        }
    }
}

impl Weather {
    fn parse(s: &str) -> Self {
        match s {
            "clear" => Weather::Clear,
            "rain" => Weather::Rain,
            "fog" => Weather::Fog,
            "snow" => Weather::Snow,
            _ => Weather::Overcast,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Overcast => "Overcast",
            Weather::Rain => "Rain",
            Weather::Fog => "Fog",
            Weather::Snow => "Snow",
        }
    }
    fn sky(self) -> Color {
        match self {
            Weather::Clear => Color::srgb(0.55, 0.68, 0.82),
            Weather::Overcast => Color::srgb(0.63, 0.66, 0.70),
            Weather::Rain => Color::srgb(0.42, 0.45, 0.50),
            Weather::Fog => Color::srgb(0.72, 0.73, 0.74),
            Weather::Snow => Color::srgb(0.80, 0.83, 0.88),
        }
    }
    /// (ambient color, ambient brightness)
    fn ambient(self) -> (Color, f32) {
        match self {
            Weather::Clear => (Color::srgb(0.85, 0.86, 0.90), 260.0),
            Weather::Overcast => (Color::srgb(0.82, 0.83, 0.85), 360.0),
            Weather::Rain => (Color::srgb(0.70, 0.72, 0.76), 320.0),
            Weather::Fog => (Color::srgb(0.86, 0.86, 0.87), 420.0),
            Weather::Snow => (Color::srgb(0.90, 0.92, 0.96), 480.0),
        }
    }
    /// (sun illuminance, casts shadows)
    fn sun(self) -> (f32, bool) {
        match self {
            Weather::Clear => (11000.0, true),
            Weather::Overcast => (4500.0, true),
            Weather::Rain => (2600.0, false),
            Weather::Fog => (2200.0, false),
            Weather::Snow => (6000.0, true),
        }
    }
    /// Optional linear fog: (color, start distance, full-fog distance). Tuned
    /// for the ~62-unit camera distance so fog reads as depth haze, not a wall.
    fn fog(self) -> Option<(Color, f32, f32)> {
        match self {
            Weather::Clear => None,
            Weather::Overcast => Some((Color::srgb(0.63, 0.66, 0.70), 72.0, 240.0)),
            Weather::Rain => Some((Color::srgb(0.45, 0.48, 0.52), 48.0, 150.0)),
            Weather::Fog => Some((Color::srgb(0.74, 0.75, 0.76), 40.0, 120.0)),
            Weather::Snow => Some((Color::srgb(0.82, 0.85, 0.90), 55.0, 175.0)),
        }
    }
    /// Ground wetness/darkening multiplier applied to terrain colors.
    fn wet(self) -> f32 {
        match self {
            Weather::Rain => 0.70,
            Weather::Fog => 0.92,
            Weather::Snow => 1.15, // snow lightens/covers the ground
            _ => 1.0,
        }
    }
}

/// A unit the player placed on the Deploy mini-map.
struct Placed {
    faction: Faction,
    kind: UnitType,
    x: f32,
    z: f32,
    group: u8,
}

/// Which command group a (player) unit belongs to.
#[derive(Component, Clone, Copy)]
struct GroupId(u8);

/// The set of command groups the player deployed units into.
#[derive(Resource, Default)]
struct PlayerGroups(Vec<u8>);

/// A command the player can issue to a group.
#[derive(Clone, Copy, PartialEq)]
enum CmdKind {
    Attack,
    Move,
    Hold,
}

/// The pending command awaiting a target click (Attack/Move).
#[derive(Resource, Default)]
struct CommandMode {
    pending: Option<CmdKind>,
}

/// Marks a group button in the command panel.
#[derive(Component)]
struct GroupButton(u8);

/// Marks a command button in the command panel.
#[derive(Component)]
struct CmdButton(CmdKind);

/// Marks the command-panel status line.
#[derive(Component)]
struct CmdStatus;

/// All player-placed units (empty → fall back to the auto-garrison).
#[derive(Resource, Default)]
struct Deployment(Vec<Placed>);

impl UnitType {
    fn parse(code: &str) -> Option<Self> {
        Some(match code {
            "inf" => UnitType::Infantry,
            "sni" => UnitType::Sniper,
            "mg" => UnitType::MachineGunner,
            "art" => UnitType::Artillery,
            "run" => UnitType::Runner,
            "sco" => UnitType::Scout,
            _ => return None,
        })
    }
}

/// Parse the compact `side:type:x:z,...` placement string from the setup page.
fn read_deployment() -> Deployment {
    let raw: String = {
        #[cfg(target_arch = "wasm32")]
        {
            read_global("__WAR_UNITS").unwrap_or_default()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            String::new()
        }
    };

    let mut out = Vec::new();
    for tok in raw.split(',').filter(|s| !s.is_empty()) {
        let mut it = tok.split(':');
        let (Some(side), Some(ty), Some(xs), Some(zs)) =
            (it.next(), it.next(), it.next(), it.next())
        else {
            continue;
        };
        let group = it.next().and_then(|g| g.parse::<u8>().ok()).unwrap_or(0);
        let faction = match side {
            "c" => Faction::Central,
            _ => Faction::British,
        };
        let (Some(kind), Ok(x), Ok(z)) = (
            UnitType::parse(ty),
            xs.parse::<f32>(),
            zs.parse::<f32>(),
        ) else {
            continue;
        };
        out.push(Placed {
            faction,
            kind,
            x,
            z,
            group,
        });
    }
    Deployment(out)
}

/// Read the setup config the HTML page stashed on `window` before booting us.
fn read_setup() -> Setup {
    #[cfg(target_arch = "wasm32")]
    {
        let loc = read_global("__WAR_LOCATION").unwrap_or_default();
        let wx = read_global("__WAR_WEATHER").unwrap_or_default();
        Setup {
            location: Location::parse(&loc),
            weather: Weather::parse(&wx),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Setup {
            location: Location::Somme,
            weather: Weather::Overcast,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn read_global(key: &str) -> Option<String> {
    let g = js_sys::global();
    js_sys::Reflect::get(&g, &wasm_bindgen::JsValue::from_str(key))
        .ok()?
        .as_string()
}

/// Scale a color's RGB by `f` (clamped), used for wet/snow terrain shifts.
fn shade(color: Color, f: f32) -> Color {
    let c = color.to_srgba();
    Color::srgb(
        (c.red * f).clamp(0.0, 1.0),
        (c.green * f).clamp(0.0, 1.0),
        (c.blue * f).clamp(0.0, 1.0),
    )
}

// ---- Battlefield geometry -------------------------------------------------

const FIELD_X: f32 = 40.0;
const FIELD_Z: f32 = 46.0;
const TRENCH_Z: f32 = 12.0; // each side's trench sits this far from centre
const LINE_HALF: f32 = 26.0; // trench runs from −LINE_HALF..+LINE_HALF in X

/// The two belligerents.
#[derive(Clone, Copy, PartialEq)]
enum Faction {
    British, // south (−Z), advancing/facing +Z
    Central, // Ottoman/German, north (+Z), facing −Z
}

impl Faction {
    /// Unit direction from this faction's trench toward the enemy.
    fn front(self) -> f32 {
        match self {
            Faction::British => 1.0,
            Faction::Central => -1.0,
        }
    }
    fn uniform(self) -> Color {
        match self {
            Faction::British => Color::srgb(0.62, 0.56, 0.36), // khaki
            Faction::Central => Color::srgb(0.36, 0.40, 0.34), // field-grey
        }
    }
    fn helmet(self) -> Color {
        match self {
            Faction::British => Color::srgb(0.30, 0.28, 0.17),
            Faction::Central => Color::srgb(0.22, 0.25, 0.20),
        }
    }
}

/// The roster of unit types the player will command.
#[derive(Clone, Copy, PartialEq, Debug)]
enum UnitType {
    Infantry,
    Sniper,
    MachineGunner,
    Artillery,
    Runner,
    Scout,
}

/// A deployed unit.
#[derive(Component)]
struct Unit {
    faction: Faction,
    kind: UnitType,
}

/// The side the human player commands. The other side is AI-driven.
const PLAYER: Faction = Faction::British;

/// Hit points.
#[derive(Component)]
struct Health {
    hp: f32,
    max: f32,
}

/// What a unit is currently trying to do.
#[derive(Clone, Copy, PartialEq)]
enum Order {
    Hold,
    MoveTo(Vec3),
    Attack(Entity),
}

/// Movement + weapon state.
#[derive(Component)]
struct Fighter {
    speed: f32,
    range: f32,
    damage: f32,
    cooldown: Timer,
    order: Order,
}

/// Marks a currently-selected (player) unit.
#[derive(Component)]
struct Selected;

/// A selection ring (child of a selected unit).
#[derive(Component)]
struct Ring;

/// A projectile in flight — a bullet tracer (straight) or an artillery shell
/// (arcing). Damage for bullets is applied hitscan at fire time, so bullets are
/// purely visual; shells deal area damage on impact.
#[derive(Component)]
struct Projectile {
    from: Vec3,
    to: Vec3,
    t: f32,
    dur: f32,
    arc: f32, // apex height; 0 for a straight bullet
    shell: bool,
    damage: f32,
    radius: f32,
    faction: Faction,
}

/// An expanding, fading explosion flash (owns its material so it can fade).
#[derive(Component)]
struct Explosion {
    life: Timer,
    max_scale: f32,
    mat: Handle<StandardMaterial>,
}

/// A tumbling debris chunk thrown by an explosion.
#[derive(Component)]
struct Debris {
    vel: Vec3,
    spin: Vec3,
    life: Timer,
}

/// A short-lived effect (muzzle flash, impact spark) that just despawns.
#[derive(Component)]
struct Ephemeral {
    life: Timer,
}

/// The live HUD line (selected count + casualties).
#[derive(Component)]
struct HudText;

/// Shared selection-ring art.
#[derive(Resource)]
struct RingArt {
    mesh: Handle<Mesh>,
    mat: Handle<StandardMaterial>,
}

/// Shared art for projectiles and explosion effects.
#[derive(Resource)]
struct ProjArt {
    bullet_mesh: Handle<Mesh>,
    bullet_mat: Handle<StandardMaterial>,
    shell_mesh: Handle<Mesh>,
    shell_mat: Handle<StandardMaterial>,
    flash_mesh: Handle<Mesh>,   // muzzle flash / spark (small sphere)
    flash_mat: Handle<StandardMaterial>,
    boom_mesh: Handle<Mesh>,    // unit sphere, scaled per explosion
    smoke_mesh: Handle<Mesh>,
    smoke_mat: Handle<StandardMaterial>,
    debris_mesh: Handle<Mesh>,
    debris_mat: Handle<StandardMaterial>,
}

/// Screen-space start of a left-drag selection box.
#[derive(Resource, Default)]
struct DragBox {
    start: Option<Vec2>,
}

/// Running casualty tally.
#[derive(Resource, Default)]
struct Casualties {
    british: u32,
    central: u32,
}

impl UnitType {
    /// (hp, move speed, weapon range, damage per shot, cooldown seconds)
    fn stats(self) -> (f32, f32, f32, f32, f32) {
        match self {
            UnitType::Infantry => (100.0, 3.2, 22.0, 18.0, 1.1),
            UnitType::Sniper => (70.0, 2.6, 40.0, 60.0, 2.6),
            UnitType::MachineGunner => (130.0, 2.0, 28.0, 10.0, 0.28),
            UnitType::Artillery => (160.0, 1.1, 60.0, 95.0, 4.2),
            UnitType::Runner => (80.0, 5.0, 0.0, 0.0, 1.0), // unarmed
            UnitType::Scout => (80.0, 4.2, 18.0, 12.0, 1.3),
        }
    }
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
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
}

const ISO_DIR: Vec3 = Vec3::new(1.0, 1.15, 1.0);
// Orthographic, so this only affects clipping and *fog distance* — kept modest
// so weather fog forms a gradient across the field rather than a solid wall.
const CAM_DIST: f32 = 62.0;

// ---- Shared art (meshes + materials built once) ---------------------------

/// Shared low-poly meshes, built once and instanced across every unit.
struct Meshes {
    legs: Handle<Mesh>,
    legs_kneel: Handle<Mesh>,
    torso: Handle<Mesh>,
    head: Handle<Mesh>,
    helmet: Handle<Mesh>,
    cap: Handle<Mesh>,
    rifle: Handle<Mesh>,
    long_rifle: Handle<Mesh>,
    satchel: Handle<Mesh>,
    binocs: Handle<Mesh>,
    mg_base: Handle<Mesh>,
    mg_barrel: Handle<Mesh>,
    mg_leg: Handle<Mesh>,
    wheel: Handle<Mesh>,
    gun_axle: Handle<Mesh>,
    gun_barrel: Handle<Mesh>,
    gun_shield: Handle<Mesh>,
    gun_trail: Handle<Mesh>,
    sandbag: Handle<Mesh>,
    duckboard: Handle<Mesh>,
}

/// Materials that are the same for both sides.
struct SharedMats {
    skin: Handle<StandardMaterial>,
    metal: Handle<StandardMaterial>,
    wood: Handle<StandardMaterial>,
    sandbag: Handle<StandardMaterial>,
    duckboard: Handle<StandardMaterial>,
}

/// Per-faction materials (uniform, helmet, cap).
struct FactionMats {
    uniform: Handle<StandardMaterial>,
    helmet: Handle<StandardMaterial>,
    cap: Handle<StandardMaterial>,
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "War.v2 — WWI trench warfare (Bevy + WebAssembly)".into(),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        // Real-time sim: keep updating even when the window loses focus, so the
        // battle never stalls (browsers still throttle fully-hidden tabs).
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .insert_resource(read_setup())
        .insert_resource(read_deployment())
        .insert_resource(ClearColor(Color::srgb(0.63, 0.67, 0.72)))
        .insert_resource(CameraFocus(Vec3::ZERO))
        .insert_resource(Rng(0x1234_5678))
        .init_resource::<DragBox>()
        .init_resource::<Casualties>()
        .init_resource::<CommandMode>()
        .add_systems(Startup, (setup_world, setup_ui, setup_command_panel).chain())
        // Input handling is chained so selection runs before command targeting.
        .add_systems(Update, (selection_input, order_input, command_targeting).chain())
        .add_systems(
            Update,
            (
                pan_camera,
                ai_command,
                combat_fire,
                projectile_flight,
                movement,
                death_cleanup,
                explosion_update,
                debris_update,
                ephemeral_cleanup,
                hud_update,
            ),
        )
        .add_systems(
            Update,
            (group_panel_click, command_panel_click, group_count_update),
        )
        .run();
}

// ---- World setup ----------------------------------------------------------

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rng: ResMut<Rng>,
    setup: Res<Setup>,
    deployment: Res<Deployment>,
) {
    let weather = setup.weather;
    let (ground_col, nomans_col, crater_col, crater_n) = setup.location.palette();
    let wet = weather.wet();

    // Sky color from the weather.
    commands.insert_resource(ClearColor(weather.sky()));

    // Camera (iso ortho) + camera-attached ambient light + optional fog.
    let (amb_col, amb_bright) = weather.ambient();
    let mut cam = commands.spawn((
        Camera3d::default(),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 40.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        Transform::from_translation(ISO_DIR.normalize() * CAM_DIST).looking_at(Vec3::ZERO, Vec3::Y),
        AmbientLight {
            color: amb_col,
            brightness: amb_bright,
            ..default()
        },
    ));
    if let Some((fog_col, start, end)) = weather.fog() {
        cam.insert(DistanceFog {
            color: fog_col,
            directional_light_color: Color::NONE,
            directional_light_exponent: 8.0,
            falloff: FogFalloff::Linear { start, end },
        });
    }

    // Sun.
    let (illum, shadows) = weather.sun();
    commands.spawn((
        DirectionalLight {
            illuminance: illum,
            shadow_maps_enabled: shadows,
            ..default()
        },
        Transform::from_xyz(30.0, 60.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground + no-man's-land, tinted by location and weather.
    let ground = meshes.add(Cuboid::new(FIELD_X * 2.0 + 8.0, 1.0, FIELD_Z * 2.0 + 8.0));
    let ground_mat = materials.add(matte(shade(ground_col, wet)));
    commands.spawn((
        Mesh3d(ground),
        MeshMaterial3d(ground_mat),
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
    let strip = meshes.add(Cuboid::new(FIELD_X * 2.0, 0.06, TRENCH_Z * 2.0 - 3.0));
    let strip_mat = materials.add(matte(shade(nomans_col, wet)));
    commands.spawn((
        Mesh3d(strip),
        MeshMaterial3d(strip_mat),
        Transform::from_xyz(0.0, 0.03, 0.0),
    ));

    // Scatter shell craters across no-man's-land (density varies by theatre).
    let crater_mat = materials.add(matte(shade(crater_col, wet)));
    for _ in 0..crater_n {
        let s = rng.range(0.6, 2.4);
        let x = rng.range(-FIELD_X, FIELD_X);
        let z = rng.range(-TRENCH_Z + 1.0, TRENCH_Z - 1.0);
        let m = meshes.add(Cuboid::new(s, rng.range(0.12, 0.4), s));
        commands.spawn((
            Mesh3d(m),
            MeshMaterial3d(crater_mat.clone()),
            Transform::from_xyz(x, 0.05, z)
                .with_rotation(Quat::from_rotation_y(rng.range(0.0, 6.28))),
        ));
    }

    // Build shared art.
    let art = build_meshes(&mut meshes);
    let shared = SharedMats {
        skin: materials.add(matte(Color::srgb(0.72, 0.56, 0.44))),
        metal: materials.add(StandardMaterial {
            base_color: Color::srgb(0.13, 0.13, 0.13),
            perceptual_roughness: 0.55,
            ..default()
        }),
        wood: materials.add(matte(Color::srgb(0.30, 0.20, 0.11))),
        sandbag: materials.add(matte(Color::srgb(0.46, 0.42, 0.28))),
        duckboard: materials.add(matte(Color::srgb(0.13, 0.11, 0.08))),
    };

    let fmats_b = make_fmats(&mut materials, Faction::British);
    let fmats_c = make_fmats(&mut materials, Faction::Central);

    // Selection-ring art (bright, unlit so it reads through the fog).
    commands.insert_resource(RingArt {
        mesh: meshes.add(Torus::new(0.85, 1.05)),
        mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 1.0, 0.45),
            emissive: LinearRgba::rgb(0.2, 0.9, 0.3),
            unlit: true,
            ..default()
        }),
    });
    // Projectile / explosion art.
    commands.insert_resource(ProjArt {
        // Short, bright tracer streak (points along +Z, its travel direction).
        bullet_mesh: meshes.add(Cuboid::new(0.05, 0.05, 0.7)),
        bullet_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.86, 0.45),
            emissive: LinearRgba::rgb(2.6, 1.7, 0.5),
            unlit: true,
            ..default()
        }),
        shell_mesh: meshes.add(Sphere::new(0.16)),
        shell_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.08, 0.08, 0.08),
            perceptual_roughness: 0.7,
            ..default()
        }),
        flash_mesh: meshes.add(Sphere::new(0.28)),
        flash_mat: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.9, 0.55),
            emissive: LinearRgba::rgb(3.0, 2.2, 0.8),
            unlit: true,
            ..default()
        }),
        boom_mesh: meshes.add(Sphere::new(1.0)),
        smoke_mesh: meshes.add(Sphere::new(1.0)),
        smoke_mat: materials.add(StandardMaterial {
            base_color: Color::srgba(0.20, 0.19, 0.17, 0.55),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        debris_mesh: meshes.add(Cuboid::new(0.16, 0.16, 0.16)),
        debris_mat: materials.add(matte(Color::srgb(0.16, 0.14, 0.10))),
    });

    // Trenches are always dug for both sides.
    for faction in [Faction::British, Faction::Central] {
        spawn_trench(&mut commands, &art, &shared, &mut rng, faction);
    }

    let mut groups: Vec<u8> = Vec::new();
    if deployment.0.is_empty() {
        // No mini-map placements: fall back to the auto-garrison.
        garrison(&mut commands, &art, &shared, &fmats_b, &mut rng, Faction::British);
        garrison(&mut commands, &art, &shared, &fmats_c, &mut rng, Faction::Central);
        groups = vec![0, 1, 2, 3]; // the garrison's role-based groups
    } else {
        // Build exactly what the player placed on the Deploy mini-map.
        for p in &deployment.0 {
            let fm = match p.faction {
                Faction::British => &fmats_b,
                Faction::Central => &fmats_c,
            };
            let facing =
                Quat::from_rotation_arc(Vec3::Z, Vec3::new(0.0, 0.0, p.faction.front()));
            spawn_unit(
                &mut commands,
                &art,
                &shared,
                fm,
                p.faction,
                p.kind,
                Vec3::new(p.x, 0.0, p.z),
                facing,
                p.group,
            );
            if p.faction == PLAYER && !groups.contains(&p.group) {
                groups.push(p.group);
            }
        }
    }
    groups.sort_unstable();
    commands.insert_resource(PlayerGroups(groups));
}

fn make_fmats(materials: &mut Assets<StandardMaterial>, faction: Faction) -> FactionMats {
    FactionMats {
        uniform: materials.add(matte(faction.uniform())),
        helmet: materials.add(matte(faction.helmet())),
        cap: materials.add(matte(Color::srgb(0.34, 0.31, 0.20))),
    }
}

fn matte(color: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: color,
        perceptual_roughness: 1.0,
        ..default()
    }
}

fn build_meshes(m: &mut Assets<Mesh>) -> Meshes {
    Meshes {
        legs: m.add(Cuboid::new(0.44, 0.62, 0.26)),
        legs_kneel: m.add(Cuboid::new(0.46, 0.32, 0.30)),
        torso: m.add(Cuboid::new(0.50, 0.68, 0.30)),
        head: m.add(Cuboid::new(0.26, 0.26, 0.26)),
        helmet: m.add(Cuboid::new(0.34, 0.16, 0.34)),
        cap: m.add(Cuboid::new(0.30, 0.13, 0.32)),
        rifle: m.add(Cuboid::new(0.07, 0.07, 0.95)),
        long_rifle: m.add(Cuboid::new(0.06, 0.06, 1.35)),
        satchel: m.add(Cuboid::new(0.30, 0.32, 0.15)),
        binocs: m.add(Cuboid::new(0.24, 0.11, 0.11)),
        mg_base: m.add(Cuboid::new(0.34, 0.20, 0.34)),
        mg_barrel: m.add(Cuboid::new(0.12, 0.12, 1.15)),
        mg_leg: m.add(Cuboid::new(0.06, 0.06, 0.75)),
        wheel: m.add(Cylinder::new(0.58, 0.16)),
        gun_axle: m.add(Cuboid::new(1.7, 0.22, 0.32)),
        gun_barrel: m.add(Cuboid::new(0.20, 0.20, 2.3)),
        gun_shield: m.add(Cuboid::new(1.3, 0.95, 0.12)),
        gun_trail: m.add(Cuboid::new(0.22, 0.16, 1.5)),
        sandbag: m.add(Cuboid::new(0.82, 0.30, 0.56)),
        duckboard: m.add(Cuboid::new(LINE_HALF * 2.0 + 2.0, 0.06, 2.0)),
    }
}

// ---- Trenches -------------------------------------------------------------

/// A sandbagged trench line for one faction, with a duckboard channel, a
/// crenellated front parapet and a lower rear parados.
fn spawn_trench(
    commands: &mut Commands,
    art: &Meshes,
    shared: &SharedMats,
    rng: &mut Rng,
    faction: Faction,
) {
    let front = faction.front();
    let z_line = -front * TRENCH_Z; // trench sits on the friendly side of centre

    // Duckboard / mud channel.
    commands.spawn((
        Mesh3d(art.duckboard.clone()),
        MeshMaterial3d(shared.duckboard.clone()),
        Transform::from_xyz(0.0, 0.05, z_line),
    ));

    // Sandbag rows. Front parapet (enemy side): taller & crenellated. Rear
    // parados: lower.
    let mut x = -LINE_HALF;
    let mut i = 0;
    while x <= LINE_HALF {
        let jx = x + rng.range(-0.06, 0.06);
        // Front parapet.
        let front_z = z_line + front * 1.35;
        let tall = i % 3 != 0; // crenellations: gaps every third bag
        commands.spawn((
            Mesh3d(art.sandbag.clone()),
            MeshMaterial3d(shared.sandbag.clone()),
            Transform::from_xyz(jx, 0.15, front_z)
                .with_rotation(Quat::from_rotation_y(rng.range(-0.08, 0.08))),
        ));
        if tall {
            commands.spawn((
                Mesh3d(art.sandbag.clone()),
                MeshMaterial3d(shared.sandbag.clone()),
                Transform::from_xyz(jx, 0.44, front_z)
                    .with_rotation(Quat::from_rotation_y(rng.range(-0.08, 0.08))),
            ));
        }
        // Rear parados (single, lower row).
        let back_z = z_line - front * 1.35;
        commands.spawn((
            Mesh3d(art.sandbag.clone()),
            MeshMaterial3d(shared.sandbag.clone()),
            Transform::from_xyz(jx, 0.13, back_z)
                .with_rotation(Quat::from_rotation_y(rng.range(-0.1, 0.1))),
        ));
        x += 0.9;
        i += 1;
    }
}

// ---- Garrison -------------------------------------------------------------

/// Deploy one faction's mixed garrison in and around its trench.
fn garrison(
    commands: &mut Commands,
    art: &Meshes,
    shared: &SharedMats,
    fmats: &FactionMats,
    rng: &mut Rng,
    faction: Faction,
) {
    let front = faction.front();
    let z_line = -front * TRENCH_Z;
    let facing = Quat::from_rotation_arc(Vec3::Z, Vec3::new(0.0, 0.0, front));

    let jitter = |rng: &mut Rng| Vec3::new(rng.range(-0.25, 0.25), 0.0, rng.range(-0.25, 0.25));

    // Infantry manning the fire trench.
    let mut x = -LINE_HALF + 1.0;
    while x <= LINE_HALF - 1.0 {
        // Machine-gun nests punctuate the line; snipers hold the flanks.
        let kind = if (x + LINE_HALF).rem_euclid(9.0) < 0.9 {
            UnitType::MachineGunner
        } else if x.abs() > LINE_HALF - 3.0 {
            UnitType::Sniper
        } else {
            UnitType::Infantry
        };
        let pos = Vec3::new(x, 0.0, z_line) + jitter(rng);
        spawn_unit(commands, art, shared, fmats, faction, kind, pos, facing, group_for_kind(kind));
        x += 2.8;
    }

    // Scouts pushed forward toward no-man's-land.
    for sx in [-8.0_f32, 8.0] {
        let pos = Vec3::new(sx, 0.0, z_line + front * 2.6) + jitter(rng);
        spawn_unit(commands, art, shared, fmats, faction, UnitType::Scout, pos, facing, group_for_kind(UnitType::Scout));
    }
    // Runners between the trench and the guns.
    for rx in [-4.0_f32, 4.0] {
        let pos = Vec3::new(rx, 0.0, z_line - front * 3.5) + jitter(rng);
        spawn_unit(commands, art, shared, fmats, faction, UnitType::Runner, pos, facing, group_for_kind(UnitType::Runner));
    }
    // Artillery battery dug in behind the line.
    for gx in [-12.0_f32, 0.0, 12.0] {
        let pos = Vec3::new(gx, 0.0, z_line - front * 7.0);
        spawn_unit(commands, art, shared, fmats, faction, UnitType::Artillery, pos, facing, group_for_kind(UnitType::Artillery));
    }
}

/// Spawn one unit of `kind` at `pos` facing `facing`, assembling its low-poly
/// model from shared meshes.
/// Default command group for the auto-garrison (by role).
fn group_for_kind(k: UnitType) -> u8 {
    match k {
        UnitType::Infantry | UnitType::Sniper => 0,
        UnitType::MachineGunner => 1,
        UnitType::Artillery => 2,
        UnitType::Scout | UnitType::Runner => 3,
    }
}

fn spawn_unit(
    commands: &mut Commands,
    art: &Meshes,
    shared: &SharedMats,
    fmats: &FactionMats,
    faction: Faction,
    kind: UnitType,
    pos: Vec3,
    facing: Quat,
    group: u8,
) {
    let (hp, speed, range, damage, cooldown) = kind.stats();
    let mut e = commands.spawn((
        Transform::from_translation(pos).with_rotation(facing),
        Visibility::default(),
        Unit { faction, kind },
        Health { hp, max: hp },
        Fighter {
            speed,
            range,
            damage,
            cooldown: Timer::from_seconds(cooldown, TimerMode::Once),
            order: Order::Hold,
        },
    ));
    // Only the player's units carry a command group.
    if faction == PLAYER {
        e.insert(GroupId(group));
    }

    match kind {
        UnitType::Infantry => add_soldier(&mut e, art, shared, fmats, Pose::Stand, Head::Helmet, Weapon::Rifle),
        UnitType::Sniper => {
            add_soldier(&mut e, art, shared, fmats, Pose::Kneel, Head::Helmet, Weapon::LongRifle)
        }
        UnitType::Runner => {
            add_soldier(&mut e, art, shared, fmats, Pose::Stand, Head::Cap, Weapon::None);
            add_satchel(&mut e, art, shared);
        }
        UnitType::Scout => {
            add_soldier(&mut e, art, shared, fmats, Pose::Stand, Head::Cap, Weapon::Rifle);
            add_binoculars(&mut e, art, shared);
        }
        UnitType::MachineGunner => {
            add_soldier(&mut e, art, shared, fmats, Pose::Kneel, Head::Helmet, Weapon::None);
            add_machine_gun(&mut e, art, shared);
        }
        UnitType::Artillery => {
            add_field_gun(&mut e, art, shared);
            // A crewman kneeling at the breech.
            add_soldier(&mut e, art, shared, fmats, Pose::Kneel, Head::Helmet, Weapon::None);
        }
    }
}

enum Pose {
    Stand,
    Kneel,
}
enum Head {
    Helmet,
    Cap,
}
enum Weapon {
    Rifle,
    LongRifle,
    None,
}

fn add_soldier(
    e: &mut EntityCommands,
    art: &Meshes,
    shared: &SharedMats,
    fmats: &FactionMats,
    pose: Pose,
    head: Head,
    weapon: Weapon,
) {
    let kneel = matches!(pose, Pose::Kneel);
    let (legs_mesh, leg_y, torso_y, head_y, hat_y, wpn_y) = if kneel {
        (art.legs_kneel.clone(), 0.16, 0.62, 1.02, 1.18, 0.62)
    } else {
        (art.legs.clone(), 0.31, 0.96, 1.40, 1.56, 0.95)
    };

    e.with_children(|c| {
        c.spawn((
            Mesh3d(legs_mesh),
            MeshMaterial3d(fmats.uniform.clone()),
            Transform::from_xyz(0.0, leg_y, 0.0),
        ));
        c.spawn((
            Mesh3d(art.torso.clone()),
            MeshMaterial3d(fmats.uniform.clone()),
            Transform::from_xyz(0.0, torso_y, 0.0),
        ));
        c.spawn((
            Mesh3d(art.head.clone()),
            MeshMaterial3d(shared.skin.clone()),
            Transform::from_xyz(0.0, head_y, 0.0),
        ));
        match head {
            Head::Helmet => {
                c.spawn((
                    Mesh3d(art.helmet.clone()),
                    MeshMaterial3d(fmats.helmet.clone()),
                    Transform::from_xyz(0.0, hat_y, 0.0),
                ));
            }
            Head::Cap => {
                c.spawn((
                    Mesh3d(art.cap.clone()),
                    MeshMaterial3d(fmats.cap.clone()),
                    Transform::from_xyz(0.0, hat_y - 0.02, 0.0),
                ));
            }
        }
        match weapon {
            Weapon::Rifle => {
                c.spawn((
                    Mesh3d(art.rifle.clone()),
                    MeshMaterial3d(shared.wood.clone()),
                    Transform::from_xyz(0.17, wpn_y, 0.30)
                        .with_rotation(Quat::from_rotation_x(-0.35)),
                ));
            }
            Weapon::LongRifle => {
                c.spawn((
                    Mesh3d(art.long_rifle.clone()),
                    MeshMaterial3d(shared.wood.clone()),
                    Transform::from_xyz(0.16, wpn_y + 0.05, 0.45)
                        .with_rotation(Quat::from_rotation_x(-0.15)),
                ));
            }
            Weapon::None => {}
        }
    });
}

fn add_satchel(e: &mut EntityCommands, art: &Meshes, shared: &SharedMats) {
    e.with_children(|c| {
        c.spawn((
            Mesh3d(art.satchel.clone()),
            MeshMaterial3d(shared.wood.clone()),
            Transform::from_xyz(0.0, 0.9, -0.24),
        ));
    });
}

fn add_binoculars(e: &mut EntityCommands, art: &Meshes, shared: &SharedMats) {
    e.with_children(|c| {
        c.spawn((
            Mesh3d(art.binocs.clone()),
            MeshMaterial3d(shared.metal.clone()),
            Transform::from_xyz(0.0, 1.40, 0.20),
        ));
    });
}

fn add_machine_gun(e: &mut EntityCommands, art: &Meshes, shared: &SharedMats) {
    e.with_children(|c| {
        c.spawn((
            Mesh3d(art.mg_base.clone()),
            MeshMaterial3d(shared.metal.clone()),
            Transform::from_xyz(0.0, 0.26, 0.55),
        ));
        c.spawn((
            Mesh3d(art.mg_barrel.clone()),
            MeshMaterial3d(shared.metal.clone()),
            Transform::from_xyz(0.0, 0.44, 1.1),
        ));
        // Splayed tripod legs.
        for (dx, rot) in [(0.22_f32, 0.4_f32), (-0.22, -0.4)] {
            c.spawn((
                Mesh3d(art.mg_leg.clone()),
                MeshMaterial3d(shared.metal.clone()),
                Transform::from_xyz(dx, 0.2, 0.35)
                    .with_rotation(Quat::from_rotation_z(rot) * Quat::from_rotation_x(0.5)),
            ));
        }
    });
}

fn add_field_gun(e: &mut EntityCommands, art: &Meshes, shared: &SharedMats) {
    e.with_children(|c| {
        // Two wheels on a horizontal axle (cylinder rotated to lie along X).
        for dx in [0.85_f32, -0.85] {
            c.spawn((
                Mesh3d(art.wheel.clone()),
                MeshMaterial3d(shared.wood.clone()),
                Transform::from_xyz(dx, 0.58, 0.0)
                    .with_rotation(Quat::from_rotation_z(FRAC_PI_2)),
            ));
        }
        c.spawn((
            Mesh3d(art.gun_axle.clone()),
            MeshMaterial3d(shared.metal.clone()),
            Transform::from_xyz(0.0, 0.6, 0.0),
        ));
        c.spawn((
            Mesh3d(art.gun_shield.clone()),
            MeshMaterial3d(shared.metal.clone()),
            Transform::from_xyz(0.0, 0.78, 0.25),
        ));
        // Barrel, angled up toward the enemy (local +Z).
        c.spawn((
            Mesh3d(art.gun_barrel.clone()),
            MeshMaterial3d(shared.metal.clone()),
            Transform::from_xyz(0.0, 0.85, 1.0)
                .with_rotation(Quat::from_rotation_x(-0.18)),
        ));
        // Trail leg out the back.
        c.spawn((
            Mesh3d(art.gun_trail.clone()),
            MeshMaterial3d(shared.wood.clone()),
            Transform::from_xyz(0.0, 0.35, -0.95),
        ));
    });
}

// ---- UI overlay -----------------------------------------------------------

fn setup_ui(mut commands: Commands, setup: Res<Setup>) {
    let subtitle = format!(
        "{}  -  {}   |   British (khaki, south)  vs  Central Powers (grey, north)",
        setup.location.name(),
        setup.weather.name(),
    );
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
                Text::new("War.v2  -  WWI Trench Warfare"),
                TextFont {
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgb(0.12, 0.12, 0.12)),
            ));
            p.spawn((
                Text::new(subtitle),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.20, 0.20, 0.20)),
            ));
            p.spawn((
                Text::new(
                    "Left-click / drag: select your troops  .  right-click: move or attack  .  WASD: pan",
                ),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.24, 0.24, 0.24)),
            ));
            p.spawn((
                HudText,
                Text::new("Selected: 0    Casualties  -  British 0  /  Central 0"),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.10, 0.10, 0.10)),
            ));
        });

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

// ---- Camera ---------------------------------------------------------------

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
        focus.0 += dir.normalize() * 26.0 * time.delta_secs();
        focus.0.x = focus.0.x.clamp(-FIELD_X, FIELD_X);
        focus.0.z = focus.0.z.clamp(-FIELD_Z, FIELD_Z);
    }

    if let Ok(mut transform) = cam.single_mut() {
        let eye = focus.0 + ISO_DIR.normalize() * CAM_DIST;
        *transform = Transform::from_translation(eye).looking_at(focus.0, Vec3::Y);
    }
}

// ---- Real-time battle -----------------------------------------------------

/// Ray-cast the cursor onto the ground plane (y = 0).
fn cursor_ground(
    windows: &Query<&Window>,
    cam: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec3> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, gt) = cam.single().ok()?;
    let ray = camera.viewport_to_world(gt, cursor).ok()?;
    let dy = ray.direction.y;
    if dy.abs() < 1e-5 {
        return None;
    }
    let t = -ray.origin.y / dy;
    if t < 0.0 {
        return None;
    }
    Some(ray.origin + *ray.direction * t)
}

fn xz_dist(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

/// True while the cursor is hovering/pressing any UI element.
fn any_ui_active(ui: &Query<&Interaction>) -> bool {
    ui.iter().any(|i| !matches!(i, Interaction::None))
}

fn clear_selection(
    commands: &mut Commands,
    selected: &Query<Entity, With<Selected>>,
    children_q: &Query<&Children>,
    rings: &Query<(), With<Ring>>,
) {
    for e in selected {
        commands.entity(e).remove::<Selected>();
        if let Ok(ch) = children_q.get(e) {
            for c in ch.iter() {
                if rings.get(c).is_ok() {
                    commands.entity(c).despawn();
                }
            }
        }
    }
}

fn select_unit(commands: &mut Commands, ring: &RingArt, e: Entity) {
    commands.entity(e).insert(Selected).with_children(|c| {
        c.spawn((
            Ring,
            Mesh3d(ring.mesh.clone()),
            MeshMaterial3d(ring.mat.clone()),
            Transform::from_xyz(0.0, 0.12, 0.0),
        ));
    });
}

/// Left-click selects the friendly unit under the cursor; left-drag box-selects.
fn selection_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cam: Query<(&Camera, &GlobalTransform)>,
    ui: Query<&Interaction>,
    mode: Res<CommandMode>,
    mut drag: ResMut<DragBox>,
    mut commands: Commands,
    ring: Res<RingArt>,
    units: Query<(Entity, &GlobalTransform, &Unit)>,
    selected: Query<Entity, With<Selected>>,
    children_q: Query<&Children>,
    rings: Query<(), With<Ring>>,
) {
    // Don't start a selection while targeting a command or clicking the panel.
    if mode.pending.is_some() || any_ui_active(&ui) {
        if mouse.just_pressed(MouseButton::Left) {
            drag.start = None;
        }
        return;
    }
    if mouse.just_pressed(MouseButton::Left) {
        drag.start = windows.single().ok().and_then(|w| w.cursor_position());
        return;
    }
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    let end = windows.single().ok().and_then(|w| w.cursor_position());
    let Some(start) = drag.start.take() else {
        return;
    };
    let Some(end) = end else {
        return;
    };

    clear_selection(&mut commands, &selected, &children_q, &rings);
    let (s, e) = (start, end);

    if s.distance(e) < 6.0 {
        // Click: nearest friendly unit to the ground point.
        if let Some(gp) = cursor_ground(&windows, &cam) {
            let mut best = None;
            let mut bd = 2.2_f32;
            for (ent, gt, u) in &units {
                if u.faction != PLAYER {
                    continue;
                }
                let d = xz_dist(gt.translation(), gp);
                if d < bd {
                    bd = d;
                    best = Some(ent);
                }
            }
            if let Some(ent) = best {
                select_unit(&mut commands, &ring, ent);
            }
        }
    } else if let Ok((camera, cgt)) = cam.single() {
        // Drag box: every friendly unit whose screen position is inside it.
        let (min, max) = (s.min(e), s.max(e));
        for (ent, gt, u) in &units {
            if u.faction != PLAYER {
                continue;
            }
            if let Ok(sp) = camera.world_to_viewport(cgt, gt.translation()) {
                if sp.x >= min.x && sp.x <= max.x && sp.y >= min.y && sp.y <= max.y {
                    select_unit(&mut commands, &ring, ent);
                }
            }
        }
    }
}

/// Right-click orders the selected units: attack an enemy under the cursor, or
/// move to the ground point in a loose formation.
fn order_input(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cam: Query<(&Camera, &GlobalTransform)>,
    ui: Query<&Interaction>,
    units: Query<(Entity, &GlobalTransform, &Unit)>,
    mut selected: Query<&mut Fighter, With<Selected>>,
) {
    if !mouse.just_pressed(MouseButton::Right) || any_ui_active(&ui) {
        return;
    }
    let Some(gp) = cursor_ground(&windows, &cam) else {
        return;
    };
    if let Some(te) = enemy_near(&units, gp) {
        for mut f in &mut selected {
            f.order = Order::Attack(te);
        }
    } else {
        issue_move(&mut selected, gp);
    }
}

/// Nearest enemy unit within a small radius of a ground point.
fn enemy_near(units: &Query<(Entity, &GlobalTransform, &Unit)>, gp: Vec3) -> Option<Entity> {
    let mut enemy = None;
    let mut bd = 2.8_f32;
    for (e, gt, u) in units {
        if u.faction == PLAYER {
            continue;
        }
        let d = xz_dist(gt.translation(), gp);
        if d < bd {
            bd = d;
            enemy = Some(e);
        }
    }
    enemy
}

/// Order the selected units to move to `gp` in a loose grid formation.
fn issue_move(selected: &mut Query<&mut Fighter, With<Selected>>, gp: Vec3) {
    let n = selected.iter().count().max(1);
    let cols = (n as f32).sqrt().ceil() as i32;
    for (i, mut f) in selected.iter_mut().enumerate() {
        let i = i as i32;
        let ox = ((i % cols) - cols / 2) as f32 * 2.0;
        let oz = ((i / cols) - cols / 2) as f32 * 2.0;
        f.order = Order::MoveTo(gp + Vec3::new(ox, 0.0, oz));
    }
}

/// Simple enemy AI: each AI unit attack-moves toward the nearest player unit.
fn ai_command(
    units: Query<(Entity, &GlobalTransform, &Unit)>,
    mut ai: Query<(&GlobalTransform, &Unit, &mut Fighter)>,
) {
    let enemies: Vec<(Entity, Vec3)> = units
        .iter()
        .filter(|(_, _, u)| u.faction == PLAYER)
        .map(|(e, gt, _)| (e, gt.translation()))
        .collect();
    if enemies.is_empty() {
        return;
    }
    for (gt, u, mut f) in &mut ai {
        if u.faction == PLAYER {
            continue;
        }
        let keep = matches!(f.order, Order::Attack(t) if enemies.iter().any(|(e, _)| *e == t));
        if keep {
            continue;
        }
        let p = gt.translation();
        let mut best = None;
        let mut bd = f32::MAX;
        for (e, ep) in &enemies {
            let d = (*ep - p).length();
            if d < bd {
                bd = d;
                best = Some(*e);
            }
        }
        if let Some(te) = best {
            f.order = Order::Attack(te);
        }
    }
}

/// Units auto-fire at the nearest enemy within weapon range, spawning a tracer
/// and dealing damage.
struct Shot {
    from: Vec3,
    to: Vec3,
    target: Entity,
    dmg: f32,
    arty: bool,
    faction: Faction,
}

fn combat_fire(
    time: Res<Time>,
    mut commands: Commands,
    art: Res<ProjArt>,
    positions: Query<(Entity, &GlobalTransform, &Unit)>,
    mut shooters: Query<(&GlobalTransform, &Unit, &mut Fighter)>,
    mut healths: Query<&mut Health>,
) {
    let dt = time.delta();
    let snap: Vec<(Entity, Vec3, Faction)> = positions
        .iter()
        .map(|(e, gt, u)| (e, gt.translation(), u.faction))
        .collect();

    let mut shots: Vec<Shot> = Vec::new();
    for (gt, u, mut f) in &mut shooters {
        f.cooldown.tick(dt);
        if f.range <= 0.0 || !f.cooldown.is_finished() {
            continue;
        }
        let p = gt.translation();
        let mut best = None;
        let mut bd = f.range;
        for (e, ep, ef) in &snap {
            if *ef == u.faction {
                continue;
            }
            let d = (*ep - p).length();
            if d <= bd {
                bd = d;
                best = Some((*e, *ep));
            }
        }
        if let Some((te, tp)) = best {
            f.cooldown.reset();
            shots.push(Shot {
                from: p,
                to: tp,
                target: te,
                dmg: f.damage,
                arty: matches!(u.kind, UnitType::Artillery),
                faction: u.faction,
            });
        }
    }

    for s in shots {
        let muzzle = s.from + Vec3::Y * 0.95;
        spawn_flash(&mut commands, &art, muzzle, if s.arty { 0.10 } else { 0.05 });

        if s.arty {
            // Arcing shell; damage is dealt as area effect on impact.
            let ground = Vec3::new(s.to.x, 0.0, s.to.z);
            let dist = xz_dist(muzzle, ground);
            commands.spawn((
                Mesh3d(art.shell_mesh.clone()),
                MeshMaterial3d(art.shell_mat.clone()),
                Transform::from_translation(muzzle),
                Projectile {
                    from: muzzle,
                    to: ground,
                    t: 0.0,
                    dur: (dist / 34.0).clamp(0.6, 1.9),
                    arc: (dist * 0.32).clamp(5.0, 16.0),
                    shell: true,
                    damage: s.dmg,
                    radius: 4.5,
                    faction: s.faction,
                },
            ));
        } else {
            // Bullet: hitscan damage now, fast tracer streak for the eye.
            if let Ok(mut h) = healths.get_mut(s.target) {
                h.hp -= s.dmg;
            }
            let b = s.to + Vec3::Y * 0.9;
            let dist = (b - muzzle).length();
            commands.spawn((
                Mesh3d(art.bullet_mesh.clone()),
                MeshMaterial3d(art.bullet_mat.clone()),
                Transform::from_translation(muzzle),
                Projectile {
                    from: muzzle,
                    to: b,
                    t: 0.0,
                    dur: (dist / 95.0).clamp(0.04, 0.34),
                    arc: 0.0,
                    shell: false,
                    damage: 0.0,
                    radius: 0.0,
                    faction: s.faction,
                },
            ));
        }
    }
}

fn spawn_flash(commands: &mut Commands, art: &ProjArt, pos: Vec3, life: f32) {
    commands.spawn((
        Mesh3d(art.flash_mesh.clone()),
        MeshMaterial3d(art.flash_mat.clone()),
        Transform::from_translation(pos),
        Ephemeral {
            life: Timer::from_seconds(life, TimerMode::Once),
        },
    ));
}

/// Advance projectiles along their path (straight for bullets, parabolic for
/// shells); on arrival, bullets spark and shells explode with area damage.
fn projectile_flight(
    time: Res<Time>,
    mut commands: Commands,
    art: Res<ProjArt>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut proj: Query<(Entity, &mut Transform, &mut Projectile)>,
    units: Query<(Entity, &GlobalTransform, &Unit)>,
    mut healths: Query<&mut Health>,
) {
    let dt = time.delta_secs();
    let snap: Vec<(Entity, Vec3, Faction)> = units
        .iter()
        .map(|(e, gt, u)| (e, gt.translation(), u.faction))
        .collect();

    for (e, mut tf, mut p) in &mut proj {
        p.t += dt;
        let frac = (p.t / p.dur).min(1.0);
        let point = |f: f32| p.from.lerp(p.to, f) + Vec3::Y * (p.arc * 4.0 * f * (1.0 - f));
        let pos = point(frac);
        let ahead = point((frac + 0.03).min(1.0)) - pos;
        if ahead.length_squared() > 1e-6 {
            tf.rotation = Quat::from_rotation_arc(Vec3::Z, ahead.normalize());
        }
        tf.translation = pos;

        if frac >= 1.0 {
            if p.shell {
                spawn_explosion(&mut commands, &art, &mut materials, p.to);
                for (te, tp, tfac) in &snap {
                    if *tfac == p.faction {
                        continue;
                    }
                    let d = xz_dist(*tp, p.to);
                    if d < p.radius {
                        if let Ok(mut h) = healths.get_mut(*te) {
                            h.hp -= p.damage * (1.0 - d / p.radius);
                        }
                    }
                }
            } else {
                spawn_flash(&mut commands, &art, pos, 0.04);
            }
            commands.entity(e).despawn();
        }
    }
}

/// Spawn an explosion: an expanding fading flash, a smoke puff, and debris.
fn spawn_explosion(
    commands: &mut Commands,
    art: &ProjArt,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
) {
    // Flash owns a fresh material so it can fade independently.
    let flash_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.7, 0.3, 1.0),
        emissive: LinearRgba::rgb(4.0, 2.2, 0.7),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(art.boom_mesh.clone()),
        MeshMaterial3d(flash_mat.clone()),
        Transform::from_translation(at + Vec3::Y * 0.6).with_scale(Vec3::splat(0.4)),
        Explosion {
            life: Timer::from_seconds(0.45, TimerMode::Once),
            max_scale: 3.4,
            mat: flash_mat,
        },
    ));
    // Lingering smoke.
    let smoke_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.22, 0.20, 0.18, 0.6),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(art.smoke_mesh.clone()),
        MeshMaterial3d(smoke_mat.clone()),
        Transform::from_translation(at + Vec3::Y * 1.2).with_scale(Vec3::splat(1.2)),
        Explosion {
            life: Timer::from_seconds(1.3, TimerMode::Once),
            max_scale: 3.0,
            mat: smoke_mat,
        },
    ));
    // Debris chunks flung outward on deterministic-ish angles.
    for i in 0..9 {
        let a = i as f32 * 0.698 + at.x * 0.3;
        let up = 7.0 + (i as f32 * 1.7).sin().abs() * 6.0;
        let out = 4.0 + (i as f32 * 0.9).cos().abs() * 4.0;
        let vel = Vec3::new(a.cos() * out, up, a.sin() * out);
        commands.spawn((
            Mesh3d(art.debris_mesh.clone()),
            MeshMaterial3d(art.debris_mat.clone()),
            Transform::from_translation(at + Vec3::Y * 0.4),
            Debris {
                vel,
                spin: Vec3::new(a.sin() * 9.0, a.cos() * 7.0, a * 3.0),
                life: Timer::from_seconds(1.4, TimerMode::Once),
            },
        ));
    }
}

/// Move each unit toward its order goal, separating so none overlap, and face
/// the direction of travel.
fn movement(time: Res<Time>, mut q: Query<(Entity, &mut Transform, &mut Fighter)>) {
    let dt = time.delta_secs();
    let snap: Vec<(Entity, Vec3)> = q.iter().map(|(e, t, _)| (e, t.translation)).collect();
    let posmap: HashMap<Entity, Vec3> = snap.iter().copied().collect();

    for (e, mut tf, mut f) in &mut q {
        let mut pos = tf.translation;

        let goal = match f.order {
            Order::Hold => None,
            Order::MoveTo(p) => {
                if xz_dist(pos, p) < 1.0 {
                    f.order = Order::Hold;
                    None
                } else {
                    Some(p)
                }
            }
            Order::Attack(te) => match posmap.get(&te) {
                Some(tp) => {
                    if xz_dist(pos, *tp) > f.range * 0.85 {
                        Some(*tp)
                    } else {
                        None // in range — hold and fire
                    }
                }
                None => {
                    f.order = Order::Hold;
                    None
                }
            },
        };

        if let Some(g) = goal {
            let mut d = g - pos;
            d.y = 0.0;
            let dist = d.length();
            if dist > 0.001 {
                pos += d.normalize() * f.speed * dt;
            }
        }

        // Separation.
        let mut push = Vec3::ZERO;
        for (oe, op) in &snap {
            if *oe == e {
                continue;
            }
            let mut d = pos - *op;
            d.y = 0.0;
            let dd = d.length();
            if dd > 0.0001 && dd < 1.1 {
                push += d.normalize() * (1.1 - dd);
            }
        }
        pos += push * 0.5;
        pos.x = pos.x.clamp(-FIELD_X, FIELD_X);
        pos.z = pos.z.clamp(-FIELD_Z, FIELD_Z);
        pos.y = 0.0;

        if let Some(g) = goal {
            let mut h = g - tf.translation;
            h.y = 0.0;
            if h.length() > 0.01 {
                let yaw = Quat::from_rotation_arc(Vec3::Z, h.normalize());
                tf.rotation = tf.rotation.slerp(yaw, 1.0 - (-6.0 * dt).exp());
            }
        }
        tf.translation = pos;
    }
}

/// Despawn units that have run out of hit points and tally the loss.
fn death_cleanup(
    mut commands: Commands,
    mut cas: ResMut<Casualties>,
    q: Query<(Entity, &Unit, &Health)>,
) {
    for (e, u, h) in &q {
        if h.hp <= 0.0 {
            match u.faction {
                Faction::British => cas.british += 1,
                Faction::Central => cas.central += 1,
            }
            commands.entity(e).despawn();
        }
    }
}

/// Grow and fade explosion flashes / smoke, then remove them.
fn explosion_update(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q: Query<(Entity, &mut Transform, &mut Explosion)>,
) {
    let dt = time.delta();
    for (e, mut tf, mut ex) in &mut q {
        ex.life.tick(dt);
        let f = ex.life.fraction();
        // Ease-out growth; fade alpha and emissive toward the end.
        let scale = ex.max_scale * (1.0 - (1.0 - f).powi(2)).max(0.05);
        tf.scale = Vec3::splat(scale.max(0.05));
        if let Some(mut m) = materials.get_mut(&ex.mat) {
            let a = (1.0 - f).clamp(0.0, 1.0);
            let base = m.base_color.to_srgba();
            m.base_color = Color::srgba(base.red, base.green, base.blue, base.alpha.max(0.6) * a);
            let em = m.emissive;
            m.emissive = LinearRgba::rgb(em.red * a, em.green * a, em.blue * a);
        }
        if ex.life.is_finished() {
            commands.entity(e).despawn();
        }
    }
}

/// Ballistic debris chunks under gravity, tumbling, settling on the ground.
fn debris_update(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &mut Debris)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut d) in &mut q {
        d.vel.y -= 24.0 * dt;
        let vel = d.vel;
        tf.translation += vel * dt;
        if tf.translation.y < 0.1 {
            tf.translation.y = 0.1;
            d.vel = Vec3::ZERO; // settled
        }
        let spin = d.spin * dt;
        tf.rotate(Quat::from_euler(EulerRot::XYZ, spin.x, spin.y, spin.z));
        if d.life.tick(time.delta()).is_finished() {
            commands.entity(e).despawn();
        }
    }
}

/// Remove short-lived effects (muzzle flashes, sparks).
fn ephemeral_cleanup(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Ephemeral)>,
) {
    for (e, mut x) in &mut q {
        if x.life.tick(time.delta()).is_finished() {
            commands.entity(e).despawn();
        }
    }
}

/// Keep the HUD line (selected count + casualties) up to date.
fn hud_update(
    cas: Res<Casualties>,
    selected: Query<(), With<Selected>>,
    mut text: Query<&mut Text, With<HudText>>,
) {
    let n = selected.iter().count();
    if let Ok(mut t) = text.single_mut() {
        *t = Text::new(format!(
            "Selected: {n}    Casualties  -  British {}  /  Central {}",
            cas.british, cas.central
        ));
    }
}

// ---- Group command panel --------------------------------------------------

fn group_color(g: u8) -> Color {
    const C: [(f32, f32, f32); 6] = [
        (0.88, 0.33, 0.23),
        (0.23, 0.56, 0.88),
        (0.26, 0.70, 0.39),
        (0.85, 0.63, 0.15),
        (0.61, 0.35, 0.71),
        (0.09, 0.71, 0.66),
    ];
    let (r, gr, b) = C[(g as usize) % 6];
    Color::srgb(r, gr, b)
}

fn group_label(g: u8) -> &'static str {
    ["A", "B", "C", "D", "E", "F"][(g as usize) % 6]
}

/// Build the bottom command panel: group buttons + Attack/Move/Hold + status.
fn setup_command_panel(mut commands: Commands, groups: Res<PlayerGroups>) {
    let panel_bg = Color::srgba(0.07, 0.07, 0.09, 0.74);
    let label = |s: &str| {
        (
            Text::new(s),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.75, 0.78)),
        )
    };

    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(panel_bg),
            ))
            .with_children(|panel| {
                // Groups row.
                panel
                    .spawn(Node {
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(label("Groups:"));
                        for &g in &groups.0 {
                            row.spawn((
                                Button,
                                GroupButton(g),
                                Node {
                                    padding: UiRect::axes(Val::Px(11.0), Val::Px(5.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BorderColor::all(group_color(g)),
                                BackgroundColor(Color::srgba(0.16, 0.16, 0.18, 0.95)),
                            ))
                            .with_children(|b| {
                                b.spawn((
                                    Text::new(group_label(g)),
                                    TextFont {
                                        font_size: FontSize::Px(15.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        }
                    });
                // Command row.
                panel
                    .spawn(Node {
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(label("Command:"));
                        for (kind, text, col) in [
                            (CmdKind::Attack, "Attack", Color::srgb(0.85, 0.30, 0.25)),
                            (CmdKind::Move, "Move", Color::srgb(0.28, 0.55, 0.85)),
                            (CmdKind::Hold, "Hold", Color::srgb(0.55, 0.55, 0.58)),
                        ] {
                            row.spawn((
                                Button,
                                CmdButton(kind),
                                Node {
                                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BorderColor::all(col),
                                BackgroundColor(Color::srgba(0.16, 0.16, 0.18, 0.95)),
                            ))
                            .with_children(|b| {
                                b.spawn((
                                    Text::new(text),
                                    TextFont {
                                        font_size: FontSize::Px(14.0),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        }
                    });
                // Status line.
                panel.spawn((
                    CmdStatus,
                    Text::new("Pick a group, then a command - or use the mouse directly."),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.65, 0.65, 0.7)),
                ));
            });
        });
}

/// Clicking a group button selects that group's living units.
fn group_panel_click(
    buttons: Query<(&Interaction, &GroupButton), Changed<Interaction>>,
    mut commands: Commands,
    ring: Res<RingArt>,
    units: Query<(Entity, &GroupId, &Unit)>,
    selected: Query<Entity, With<Selected>>,
    children_q: Query<&Children>,
    rings: Query<(), With<Ring>>,
    mut status: Query<&mut Text, With<CmdStatus>>,
) {
    for (inter, gb) in &buttons {
        if *inter != Interaction::Pressed {
            continue;
        }
        clear_selection(&mut commands, &selected, &children_q, &rings);
        let mut n = 0;
        for (e, gid, u) in &units {
            if u.faction == PLAYER && gid.0 == gb.0 {
                select_unit(&mut commands, &ring, e);
                n += 1;
            }
        }
        if let Ok(mut t) = status.single_mut() {
            *t = Text::new(format!(
                "Group {} selected ({} units) - choose a command.",
                group_label(gb.0),
                n
            ));
        }
    }
}

/// Clicking a command button: Hold acts now; Attack/Move await a target click.
fn command_panel_click(
    buttons: Query<(&Interaction, &CmdButton), Changed<Interaction>>,
    mut mode: ResMut<CommandMode>,
    mut selected: Query<&mut Fighter, With<Selected>>,
    mut status: Query<&mut Text, With<CmdStatus>>,
) {
    for (inter, cb) in &buttons {
        if *inter != Interaction::Pressed {
            continue;
        }
        let msg = match cb.0 {
            CmdKind::Hold => {
                for mut f in &mut selected {
                    f.order = Order::Hold;
                }
                mode.pending = None;
                "Holding position."
            }
            CmdKind::Attack => {
                mode.pending = Some(CmdKind::Attack);
                "ATTACK - click the target on the field."
            }
            CmdKind::Move => {
                mode.pending = Some(CmdKind::Move);
                "MOVE - click the destination on the field."
            }
        };
        if let Ok(mut t) = status.single_mut() {
            *t = Text::new(msg);
        }
    }
}

/// With a command pending, a left-click on the field resolves it.
fn command_targeting(
    mouse: Res<ButtonInput<MouseButton>>,
    ui: Query<&Interaction>,
    windows: Query<&Window>,
    cam: Query<(&Camera, &GlobalTransform)>,
    mut mode: ResMut<CommandMode>,
    units: Query<(Entity, &GlobalTransform, &Unit)>,
    mut selected: Query<&mut Fighter, With<Selected>>,
    mut status: Query<&mut Text, With<CmdStatus>>,
) {
    let Some(cmd) = mode.pending else {
        return;
    };
    if !mouse.just_pressed(MouseButton::Left) || any_ui_active(&ui) {
        return;
    }
    let Some(gp) = cursor_ground(&windows, &cam) else {
        return;
    };
    let msg = match cmd {
        CmdKind::Move => {
            issue_move(&mut selected, gp);
            "Advancing."
        }
        CmdKind::Attack => {
            if let Some(te) = enemy_near(&units, gp) {
                for mut f in &mut selected {
                    f.order = Order::Attack(te);
                }
                "Engaging the target."
            } else {
                issue_move(&mut selected, gp); // attack-move onto empty ground
                "Attacking toward that position."
            }
        }
        CmdKind::Hold => "",
    };
    mode.pending = None;
    if let Ok(mut t) = status.single_mut() {
        *t = Text::new(msg);
    }
}

/// Keep each group button's label in sync with its surviving unit count.
fn group_count_update(
    buttons: Query<(&GroupButton, &Children)>,
    mut texts: Query<&mut Text>,
    units: Query<(&GroupId, &Unit)>,
) {
    for (gb, children) in &buttons {
        let n = units
            .iter()
            .filter(|(gid, u)| u.faction == PLAYER && gid.0 == gb.0)
            .count();
        for c in children.iter() {
            if let Ok(mut t) = texts.get_mut(c) {
                *t = Text::new(format!("{} ({})", group_label(gb.0), n));
            }
        }
    }
}
