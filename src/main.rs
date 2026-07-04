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
use bevy::pbr::{DistanceFog, FogFalloff};
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
}

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
        out.push(Placed { faction, kind, x, z });
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
#[allow(dead_code)]
struct Unit {
    faction: Faction,
    kind: UnitType,
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
        .insert_resource(read_setup())
        .insert_resource(read_deployment())
        .insert_resource(ClearColor(Color::srgb(0.63, 0.67, 0.72)))
        .insert_resource(CameraFocus(Vec3::ZERO))
        .insert_resource(Rng(0x1234_5678))
        .add_systems(Startup, (setup_world, setup_ui))
        .add_systems(Update, pan_camera)
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

    // Trenches are always dug for both sides.
    for faction in [Faction::British, Faction::Central] {
        spawn_trench(&mut commands, &art, &shared, &mut rng, faction);
    }

    if deployment.0.is_empty() {
        // No mini-map placements: fall back to the auto-garrison.
        garrison(&mut commands, &art, &shared, &fmats_b, &mut rng, Faction::British);
        garrison(&mut commands, &art, &shared, &fmats_c, &mut rng, Faction::Central);
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
            );
        }
    }
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
        spawn_unit(commands, art, shared, fmats, faction, kind, pos, facing);
        x += 2.8;
    }

    // Scouts pushed forward toward no-man's-land.
    for sx in [-8.0_f32, 8.0] {
        let pos = Vec3::new(sx, 0.0, z_line + front * 2.6) + jitter(rng);
        spawn_unit(commands, art, shared, fmats, faction, UnitType::Scout, pos, facing);
    }
    // Runners between the trench and the guns.
    for rx in [-4.0_f32, 4.0] {
        let pos = Vec3::new(rx, 0.0, z_line - front * 3.5) + jitter(rng);
        spawn_unit(commands, art, shared, fmats, faction, UnitType::Runner, pos, facing);
    }
    // Artillery battery dug in behind the line.
    for gx in [-12.0_f32, 0.0, 12.0] {
        let pos = Vec3::new(gx, 0.0, z_line - front * 7.0);
        spawn_unit(commands, art, shared, fmats, faction, UnitType::Artillery, pos, facing);
    }
}

/// Spawn one unit of `kind` at `pos` facing `facing`, assembling its low-poly
/// model from shared meshes.
fn spawn_unit(
    commands: &mut Commands,
    art: &Meshes,
    shared: &SharedMats,
    fmats: &FactionMats,
    faction: Faction,
    kind: UnitType,
    pos: Vec3,
    facing: Quat,
) {
    let mut e = commands.spawn((
        Transform::from_translation(pos).with_rotation(facing),
        Visibility::default(),
        Unit { faction, kind },
    ));

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
                Text::new("Units: infantry . snipers . machine guns . artillery . runners . scouts"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.24, 0.24, 0.24)),
            ));
            p.spawn((
                Text::new("WASD / arrows: pan the front"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.28, 0.28, 0.28)),
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
