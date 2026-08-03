use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::*,
    prelude::*,
    sprite::Anchor,
    time::common_conditions::on_timer,
    window::PrimaryWindow,
    window::{PresentMode, WindowMode, WindowResolution},
};
use rand::RngExt;
use std::{f32::consts::FRAC_PI_2, ops::Mul, time::Duration};
use wasm_bindgen::prelude::*;

const BOID_SCALE: f32 = 0.28;
const BOID_SPRITE_SCALE: f32 = 6.0;
const BOID_WAKE_PER_SECOND: u32 = 20;
const BOID_SPAWN_COUNT: usize = 5;
const BOID_SPAWN_JITTER: f32 = 20.0;
const WINDOW_BORDER_COLLISION: bool = false;

/// Flocking knobs, tunable at runtime. Forces are steering weights in velocity
/// units per second, so they stay comparable with `max_velocity` rather than
/// with raw pixel distances. Build with `--features inspector` for live sliders.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct BoidParams {
    pub max_force: f32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub alignment: f32,
    pub cohesion: f32,
    pub separation: f32,
    pub separation_distance: f32,
    pub perception: f32,
    pub group_size: usize,
    pub speed: f32,
    pub rotation: f32,
}

impl Default for BoidParams {
    fn default() -> Self {
        Self {
            max_force: 4.0,
            max_velocity: 1.4,
            min_velocity: 0.8,
            alignment: 1.2,
            cohesion: 1.0,
            separation: 1.6,
            separation_distance: 25.0,
            perception: 70.0,
            group_size: 20,
            speed: 200.0,
            rotation: 5.0,
        }
    }
}

#[derive(Resource)]
struct BoidCounter {
    pub count: usize,
}

#[inline]
pub fn random_f32() -> f32 {
    rand::random()
}

#[derive(Debug, Component)]
pub struct Boid {
    pub velocity: Vec2,
    pub acceleration: Vec2,
}

#[wasm_bindgen(start)]
fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    // tracing_wasm::set_as_global_default();

    info!("start");

    let mut app = App::new();

    app.add_plugins(
            DefaultPlugins
                .build()
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Bitoids".to_string(),
                        resolution: WindowResolution::new(1980, 1200),
                        mode: WindowMode::Windowed,
                        position: WindowPosition::Automatic,
                        present_mode: PresentMode::Fifo,
                        resizable: true,
                        // fit_canvas_to_parent: true,
                        // canvas: Some("#canvas".to_string()),
                        canvas: Default::default(),
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter:
                        "info,wgpu=error,wgpu_core=warn,wgpu_hal=warn,naga=error,bevy_render=error,bevy_ecs=warn"
                            .to_string(),
                            ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .init_resource::<BoidParams>()
        .register_type::<BoidParams>()
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_handler)
        .add_systems(
            Update,
            counter_system.run_if(on_timer(Duration::from_millis(250))),
        )
        // Steering reads positions, movement writes them, wrapping corrects them.
        // Without the chain Bevy is free to run these in any order each frame.
        .add_systems(
            Update,
            (
                boid_acceleration_system
                    .run_if(on_timer(Duration::from_secs_f32(1. / 60.))),
                boid_move_system,
                collision_system,
            )
                .chain(),
        );

    // EguiPlugin must land before the inspector: the quick plugin panics otherwise.
    #[cfg(feature = "inspector")]
    app.add_plugins((
        bevy_inspector_egui::bevy_egui::EguiPlugin::default(),
        bevy_inspector_egui::quick::ResourceInspectorPlugin::<BoidParams>::default(),
    ));

    app.run();

    Ok(())
}

// #[derive(Deref)]
// struct BirdTexture(Handle<Image>);

#[derive(Component)]
struct CountText;

#[derive(Component)]
struct FpsText;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn((Camera2d, Msaa::default()));

    let font: FontSource = asset_server.load("fonts/FiraSans-Bold.ttf").into();
    let ship_atlas = load_ships_atlas(&asset_server, texture_atlases);
    commands.insert_resource(ship_atlas);

    commands.insert_resource(BoidCounter { count: 0 });

    let label_font = TextFont {
        font: font.clone(),
        font_size: FontSize::Px(40.0),
        ..default()
    };
    let label_color = TextColor(Color::srgb(0.0, 1.0, 0.0));
    let value_color = TextColor(Color::srgb(0.0, 1.0, 1.0));

    commands
        .spawn((
            Text::new("Boid Count: "),
            label_font.clone(),
            label_color,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                TextSpan::default(),
                label_font.clone(),
                value_color,
                CountText,
            ));
            parent.spawn((
                TextSpan::new("\nAverage FPS: "),
                label_font.clone(),
                label_color,
            ));
            parent.spawn((TextSpan::default(), label_font.clone(), value_color, FpsText));
        });
}

#[derive(Resource)]
struct ShipAtlas {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

fn load_ships_atlas(
    asset_server: &AssetServer,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) -> ShipAtlas {
    let texture_handle = asset_server.load("ships001.png");
    let texture_atlas =
        TextureAtlasLayout::from_grid(UVec2::new(14, 14), 32, 16, Some(UVec2::new(2, 2)), None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    ShipAtlas {
        image: texture_handle,
        layout: texture_atlas_handle,
    }
}

fn mouse_handler(
    mut commands: Commands,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut counter: ResMut<BoidCounter>,
    ship_atlas: Res<ShipAtlas>,
    params: Res<BoidParams>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    if mouse_button_input.pressed(MouseButton::Left) {
        spawn_boids(
            &mut commands,
            window,
            &mut counter,
            BOID_SPAWN_COUNT,
            ship_atlas,
            &params,
        );
    }
}

fn spawn_boids(
    commands: &mut Commands,
    window: &Window,
    counter: &mut BoidCounter,
    spawn_count: usize,
    ship_atlas: Res<ShipAtlas>,
    params: &BoidParams,
) {
    let mut rng = rand::rng();
    let boid_x = rng.random::<f32>() * window.width() - window.width() / 2.0;
    let boid_y = rng.random::<f32>() * window.height() - window.height() / 2.0;

    for count in 0..spawn_count {
        let boid_z = (counter.count + count) as f32 * 0.00001;
        // Scatter the batch: boids sharing one exact position have no separation direction.
        let jitter = vec2(
            (rng.random::<f32>() - 0.5) * BOID_SPAWN_JITTER,
            (rng.random::<f32>() - 0.5) * BOID_SPAWN_JITTER,
        );

        commands.spawn((
            Sprite::from_atlas_image(
                ship_atlas.image.clone(),
                TextureAtlas {
                    layout: ship_atlas.layout.clone(),
                    index: rng.random_range(0..16 * 32),
                },
            ),
            Anchor::TOP_CENTER,
            Transform {
                translation: Vec3::new(boid_x + jitter.x, boid_y + jitter.y, boid_z),
                scale: Vec3::splat(BOID_SCALE * BOID_SPRITE_SCALE),
                ..default()
            },
            Boid {
                acceleration: vec2(random_f32() - 0.5, random_f32() - 0.5),
                velocity: vec2(
                    rng.random::<f32>() * params.max_velocity - (params.max_velocity * 0.5),
                    rng.random::<f32>() * params.max_velocity - (params.max_velocity * 0.5),
                ),
            },
        ));
    }
    counter.count += spawn_count;
}

pub fn collision_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    boid_query: Query<(&mut Boid, &mut Transform)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };

    if WINDOW_BORDER_COLLISION {
        window_bounce_collision_system(window, boid_query);
    } else {
        window_teleport_collision_system(window, boid_query);
    }
}

fn window_bounce_collision_system(
    window: &Window,
    mut boid_query: Query<(&mut Boid, &mut Transform)>,
) {
    let half_width = window.width() * 0.5;
    let half_height = window.height() * 0.5;
    let half_size = BOID_SCALE / 2.0;

    for (mut boid, transform) in boid_query.iter_mut() {
        let x_vel = boid.velocity.x;
        let y_vel = boid.velocity.y;
        let x_pos = transform.translation.x;
        let y_pos = transform.translation.y;

        if (x_vel > 0.0 && x_pos + half_size > half_width)
            || (x_vel <= 0.0 && x_pos - half_size < -(half_width))
        {
            boid.velocity.x = -x_vel;
        }
        if y_vel < 0.0 && y_pos - half_size < -half_height {
            boid.velocity.y = -y_vel;
        }
        if y_pos + half_size > half_height && y_vel > 0.0 {
            boid.velocity.y = 0.0;
        }
    }
}

fn window_teleport_collision_system(
    window: &Window,
    mut boid_query: Query<(&mut Boid, &mut Transform)>,
) {
    let half_width = window.width() * 0.5;
    let half_height = window.height() * 0.5;

    for (_, mut transform) in boid_query.iter_mut() {
        let x_pos = transform.translation.x;
        let y_pos = transform.translation.y;
        if x_pos > half_width {
            transform.translation.x = -half_width;
        } else if x_pos < -half_width {
            transform.translation.x = half_width;
        }

        if y_pos > half_height {
            transform.translation.y = -half_height;
        } else if y_pos < -half_height {
            transform.translation.y = half_height
        }
    }
}

fn counter_system(
    diagnostics: Res<DiagnosticsStore>,
    counter: Res<BoidCounter>,
    mut count_query: Query<&mut TextSpan, (With<CountText>, Without<FpsText>)>,
    mut fps_query: Query<&mut TextSpan, With<FpsText>>,
) {
    if counter.is_changed() {
        if let Ok(mut span) = count_query.single_mut() {
            **span = format!("{}", counter.count);
        }
    }

    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            if let Ok(mut span) = fps_query.single_mut() {
                **span = format!("{:.2}", average);
            }
        }
    };
}

fn boid_move_system(
    time: Res<Time>,
    params: Res<BoidParams>,
    mut query: Query<(&mut Boid, &mut Transform)>,
) {
    let delta = time.delta_secs();
    for (mut boid, mut transform) in query.iter_mut() {
        let steered = boid.velocity + boid.acceleration.mul(delta);
        let velocity = set_velocity(params.max_velocity, params.min_velocity, &steered);
        boid.velocity = velocity;
        transform.translation += vec3(velocity.x, velocity.y, 0.0).mul(delta * params.speed);

        let angle = velocity.y.atan2(velocity.x) + FRAC_PI_2 * 3.0;
        transform.rotation = transform.rotation.slerp(
            Quat::from_axis_angle(Vec3::Z, angle),
            (delta * params.rotation).min(1.0),
        );
    }
}

pub struct BoidObject {
    pub id: u32,
    pub pos: Vec2,
    pub velocity: Vec2,
}

/// Uniform bucket grid over the wrapping play field. Cells are at least one
/// perception radius wide, so every possible neighbour lives in the 3x3 block
/// around a boid's own cell. Built with a counting sort: two flat allocations
/// per rebuild, no per-cell `Vec`.
struct SpatialGrid {
    cols: usize,
    rows: usize,
    cell: Vec2,
    world: Vec2,
    cell_start: Vec<u32>,
    items: Vec<u32>,
}

impl SpatialGrid {
    fn build(boids: &[BoidObject], world: Vec2, perception: f32) -> Self {
        // At least 3 columns and rows, otherwise the 3x3 block would visit the
        // same cell twice and count those neighbours twice.
        let cols = ((world.x / perception) as usize).max(3);
        let rows = ((world.y / perception) as usize).max(3);

        let mut grid = Self {
            cols,
            rows,
            cell: vec2(world.x / cols as f32, world.y / rows as f32),
            world,
            cell_start: vec![0; cols * rows + 1],
            items: vec![0; boids.len()],
        };

        for boid in boids.iter() {
            let cell = grid.cell_of(boid.pos);
            grid.cell_start[cell + 1] += 1;
        }
        for i in 1..grid.cell_start.len() {
            grid.cell_start[i] += grid.cell_start[i - 1];
        }

        let mut cursor = grid.cell_start.clone();
        for (index, boid) in boids.iter().enumerate() {
            let cell = grid.cell_of(boid.pos);
            grid.items[cursor[cell] as usize] = index as u32;
            cursor[cell] += 1;
        }

        grid
    }

    fn cell_of(&self, pos: Vec2) -> usize {
        let col = ((pos.x + self.world.x * 0.5) / self.cell.x).floor() as isize;
        let row = ((pos.y + self.world.y * 0.5) / self.cell.y).floor() as isize;
        let col = col.rem_euclid(self.cols as isize) as usize;
        let row = row.rem_euclid(self.rows as isize) as usize;
        row * self.cols + col
    }

    /// Collects every boid in the 3x3 block around `pos`, wrapping at the field
    /// edges so a flock straddling the seam still sees itself as one flock.
    fn collect_nearby(&self, pos: Vec2, out: &mut Vec<u32>) {
        let center = self.cell_of(pos);
        let col = (center % self.cols) as isize;
        let row = (center / self.cols) as isize;

        for row_offset in -1..=1 {
            let r = (row + row_offset).rem_euclid(self.rows as isize) as usize;
            for col_offset in -1..=1 {
                let c = (col + col_offset).rem_euclid(self.cols as isize) as usize;
                let cell = r * self.cols + c;
                let range = self.cell_start[cell] as usize..self.cell_start[cell + 1] as usize;
                out.extend_from_slice(&self.items[range]);
            }
        }
    }

    /// Shortest displacement between two points on the wrapping field.
    fn wrap_delta(&self, delta: Vec2) -> Vec2 {
        delta - self.world * (delta / self.world).round()
    }
}

fn boid_acceleration_system(
    params: Res<BoidParams>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut group_id: Local<u32>,
    mut candidates: Local<Vec<u32>>,
    mut query: Query<(Entity, &mut Boid, &Transform)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let world = vec2(window.width(), window.height());

    *group_id = group_id.wrapping_add(1);
    let wake_slot = *group_id % BOID_WAKE_PER_SECOND;

    let boids = query
        .iter()
        .map(|(entity, boid, transform)| BoidObject {
            id: entity.index().index(),
            pos: transform.translation.truncate(),
            velocity: boid.velocity,
        })
        .collect::<Vec<BoidObject>>();
    let grid = SpatialGrid::build(&boids, world, params.perception);

    let perception_2 = params.perception * params.perception;
    let separation_2 = params.separation_distance * params.separation_distance;

    for (entity, mut boid, transform) in query.iter_mut() {
        let entity_id = entity.index().index();
        if entity_id % BOID_WAKE_PER_SECOND != wake_slot {
            continue;
        }

        let pos = transform.translation.truncate();
        candidates.clear();
        grid.collect_nearby(pos, &mut candidates);

        // One pass feeds all three rules: the heading to match, the offsets to
        // close on, and the crowding to escape. Summing offsets rather than
        // absolute positions is what keeps cohesion sane across the wrap seam.
        let mut heading_sum = Vec2::ZERO;
        let mut offset_sum = Vec2::ZERO;
        let mut push_away = Vec2::ZERO;
        let mut neighbors = 0;

        for &index in candidates.iter() {
            let other = &boids[index as usize];
            if other.id == entity_id {
                continue;
            }

            let offset = grid.wrap_delta(other.pos - pos);
            let distance_2 = offset.length_squared();
            if distance_2 > perception_2 {
                continue;
            }

            heading_sum += other.velocity;
            offset_sum += offset;
            if distance_2 <= separation_2 {
                // Weight by 1/distance so the closest crowding dominates;
                // coincident boids have no direction to flee and are skipped.
                if let Some(direction) = offset.try_normalize() {
                    push_away -= direction / distance_2.sqrt();
                }
            }

            neighbors += 1;
            // Hard cap so a pile-up in one cell cannot degenerate into O(N^2).
            if neighbors >= params.group_size {
                break;
            }
        }

        if neighbors == 0 {
            boid.acceleration = Vec2::ZERO;
            continue;
        }

        let steering = steer_towards(heading_sum, boid.velocity, &params) * params.alignment
            + steer_towards(offset_sum, boid.velocity, &params) * params.cohesion
            + steer_towards(push_away, boid.velocity, &params) * params.separation;

        boid.acceleration = set_max_acc(params.max_force, &steering);
    }
}

/// Reynolds steering: the force that turns `velocity` towards `desired`, capped
/// at `max_force`. Magnitude of `desired` is irrelevant, only its direction.
fn steer_towards(desired: Vec2, velocity: Vec2, params: &BoidParams) -> Vec2 {
    let Some(direction) = desired.try_normalize() else {
        return Vec2::ZERO;
    };
    set_max_acc(
        params.max_force,
        &(direction * params.max_velocity - velocity),
    )
}

fn set_max_acc(max_acc: f32, acc: &Vec2) -> Vec2 {
    let acc_len = acc.length_squared();

    let mut new_acc = *acc;

    if acc_len > max_acc * max_acc {
        new_acc = acc.normalize_or_zero();
        new_acc = new_acc.mul(max_acc);
    }
    new_acc
}

fn set_velocity(max_vel: f32, min_vel: f32, vel: &Vec2) -> Vec2 {
    let vel_len = vel.length_squared();

    let mut new_vel = *vel;

    if vel_len > max_vel * max_vel {
        new_vel = vel.normalize_or_zero();
        new_vel = new_vel.mul(max_vel);
    } else if vel_len < min_vel * min_vel {
        new_vel = vel.normalize_or_zero();
        new_vel = new_vel.mul(min_vel);
    }
    new_vel
}
