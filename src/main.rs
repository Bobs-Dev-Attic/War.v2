//! War.v2 — a small interactive Bevy scene, compiled to WebAssembly.
//!
//! The goal of this project is to prove out the full Bevy -> WASM -> Vercel
//! pipeline: a real Bevy `App` with the render loop, sprites, transforms,
//! input handling and text, running in the browser as static files.

use bevy::color::palettes::css;
use bevy::prelude::*;

/// A floating "card" that drifts around and bounces off the window edges.
#[derive(Component)]
struct Card {
    velocity: Vec2,
    spin: f32,
}

/// Half-extents used for edge collision. Matches the visual card size.
const CARD_HALF: Vec2 = Vec2::new(70.0, 100.0);
/// Logical play area half-size (matches the default 1280x720 window-ish).
const BOUNDS: Vec2 = Vec2::new(600.0, 340.0);

fn main() {
    // In the browser, forward Rust panics to the JS console.
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "War.v2 — Bevy on the web".into(),
                        canvas: Some("#bevy-canvas".into()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::srgb(0.06, 0.09, 0.16)))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_cards, spawn_on_click))
        .run();
}

/// A palette of suit-ish colors for the floating cards.
const COLORS: [Srgba; 4] = [css::CRIMSON, css::GOLD, css::DODGER_BLUE, css::LIME_GREEN];

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Title text, anchored in world space near the top.
    commands.spawn((
        Text2d::new("War.v2 — running on Bevy + WebAssembly"),
        TextFont {
            font_size: 34.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, 290.0, 10.0),
    ));

    commands.spawn((
        Text2d::new("click anywhere to deal more cards"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.6, 0.7, 0.85)),
        Transform::from_xyz(0.0, 250.0, 10.0),
    ));

    // Deal an opening hand of cards spread across the play area.
    for i in 0..8 {
        let angle = i as f32 * std::f32::consts::TAU / 8.0;
        let pos = Vec2::new(angle.cos(), angle.sin()) * 180.0;
        let color = COLORS[i % COLORS.len()];
        spawn_card(&mut commands, pos, color, i as f32);
    }
}

/// Spawn a single card sprite with a pseudo-random-ish velocity derived from `seed`.
fn spawn_card(commands: &mut Commands, pos: Vec2, color: Srgba, seed: f32) {
    // Deterministic "randomness" from the seed so we don't need an RNG crate.
    let vx = (seed * 1.9 + 0.5).sin() * 160.0;
    let vy = (seed * 2.7 + 1.3).cos() * 160.0;
    let spin = (seed * 0.7).sin() * 1.5;

    commands.spawn((
        Sprite {
            color: color.into(),
            custom_size: Some(CARD_HALF * 2.0),
            ..default()
        },
        Transform::from_xyz(pos.x, pos.y, 0.0),
        Card {
            velocity: Vec2::new(vx, vy),
            spin,
        },
    ));
}

/// Move every card, spinning it and bouncing it off the play-area edges.
fn move_cards(time: Res<Time>, mut query: Query<(&mut Transform, &mut Card)>) {
    let dt = time.delta_secs();
    for (mut transform, mut card) in &mut query {
        transform.translation.x += card.velocity.x * dt;
        transform.translation.y += card.velocity.y * dt;
        transform.rotate_z(card.spin * dt);

        let limit = BOUNDS - CARD_HALF;
        if transform.translation.x.abs() > limit.x {
            transform.translation.x = transform.translation.x.clamp(-limit.x, limit.x);
            card.velocity.x = -card.velocity.x;
        }
        if transform.translation.y.abs() > limit.y {
            transform.translation.y = transform.translation.y.clamp(-limit.y, limit.y);
            card.velocity.y = -card.velocity.y;
        }
    }
}

/// On a left click, deal a new card at the cursor's world position.
fn spawn_on_click(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_transform)) = cameras.single() else {
        return;
    };
    if let Ok(world) = camera.viewport_to_world_2d(cam_transform, cursor) {
        let seed = time.elapsed_secs() * 3.3;
        let color = COLORS[(seed as usize) % COLORS.len()];
        spawn_card(&mut commands, world, color, seed);
    }
}
