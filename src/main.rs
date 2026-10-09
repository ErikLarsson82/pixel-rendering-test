//! Pixel rendering test bed.
//!
//! Controls:
//!   F - toggle borderless fullscreen / windowed
//!   S - cycle scaling mode (Integer -> Fit -> Stretch)
//!   N - toggle texture sampling (Nearest <-> Linear)

use bevy::camera::ScalingMode;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode, WindowResolution};

/// Native resolution of the art. Everything is laid out in these units.
const VIRTUAL_WIDTH: f32 = 1280.0;
const VIRTUAL_HEIGHT: f32 = 720.0;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                // Nearest-neighbor sampling for all images by default: crisp pixels.
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Pixel rendering test".into(),
                        // Override OS DPI scaling so 1 image pixel == 1 screen pixel in windowed mode.
                        resolution: WindowResolution::new(VIRTUAL_WIDTH as u32, VIRTUAL_HEIGHT as u32)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<RenderSettings>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (toggle_fullscreen, cycle_scaling, toggle_sampling, update_camera_scale).chain(),
        )
        .run();
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ScaleMode {
    /// Largest whole-number multiple that fits. Every source pixel becomes an
    /// identical NxN block; letterboxed. Best for pixel art.
    #[default]
    Integer,
    /// Scale to fit while keeping aspect ratio. Fractional factors make some
    /// pixels wider than others when using nearest sampling.
    Fit,
    /// Fill the window, ignoring aspect ratio.
    Stretch,
}

#[derive(Resource)]
struct RenderSettings {
    scale_mode: ScaleMode,
    nearest: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self { scale_mode: ScaleMode::default(), nearest: true }
    }
}

#[derive(Resource)]
struct Background(Handle<Image>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load("image-test.png");
    commands.spawn(Sprite::from_image(image.clone()));
    commands.insert_resource(Background(image));

    info!("Controls: F = fullscreen, S = scaling mode, N = nearest/linear sampling");
}

fn toggle_fullscreen(
    keys: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }
    window.mode = match window.mode {
        WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        _ => WindowMode::Windowed,
    };
}

fn cycle_scaling(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<RenderSettings>) {
    if !keys.just_pressed(KeyCode::KeyS) {
        return;
    }
    settings.scale_mode = match settings.scale_mode {
        ScaleMode::Integer => ScaleMode::Fit,
        ScaleMode::Fit => ScaleMode::Stretch,
        ScaleMode::Stretch => ScaleMode::Integer,
    };
    info!("Scale mode: {:?}", settings.scale_mode);
}

fn toggle_sampling(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<RenderSettings>,
    background: Res<Background>,
    mut images: ResMut<Assets<Image>>,
) {
    if !keys.just_pressed(KeyCode::KeyN) {
        return;
    }
    settings.nearest = !settings.nearest;
    if let Some(mut image) = images.get_mut(&background.0) {
        image.sampler = if settings.nearest { ImageSampler::nearest() } else { ImageSampler::linear() };
    }
    info!("Sampling: {}", if settings.nearest { "nearest" } else { "linear" });
}

/// Keeps the 1280x720 virtual canvas mapped onto the window according to the scale mode.
fn update_camera_scale(
    settings: Res<RenderSettings>,
    window: Single<Ref<Window>, With<PrimaryWindow>>,
    mut projection: Single<&mut Projection, With<Camera2d>>,
) {
    // Only recompute when something relevant changed (resize, fullscreen, mode switch).
    if !settings.is_changed() && !window.is_changed() {
        return;
    }
    let Projection::Orthographic(ortho) = projection.as_mut() else {
        return;
    };

    let (scaling_mode, scale) = match settings.scale_mode {
        ScaleMode::Integer => {
            // Work in physical pixels so the multiple is exact regardless of DPI.
            let physical = window.physical_size().as_vec2();
            let factor = (physical.x / VIRTUAL_WIDTH)
                .min(physical.y / VIRTUAL_HEIGHT)
                .floor()
                .max(1.0);
            // WindowSize: 1 world unit == 1 logical pixel at scale 1.0.
            // We want 1 world unit == `factor` physical pixels.
            (ScalingMode::WindowSize, window.scale_factor() / factor)
        }
        ScaleMode::Fit => (
            ScalingMode::AutoMin { min_width: VIRTUAL_WIDTH, min_height: VIRTUAL_HEIGHT },
            1.0,
        ),
        ScaleMode::Stretch => (
            ScalingMode::Fixed { width: VIRTUAL_WIDTH, height: VIRTUAL_HEIGHT },
            1.0,
        ),
    };

    ortho.scaling_mode = scaling_mode;
    ortho.scale = scale;
}
