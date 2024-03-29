use bevy::utils::Duration;
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::*,
    prelude::*,
    time::common_conditions::on_timer,
    window::PrimaryWindow,
    window::{PresentMode, WindowMode},
};
use rand::{thread_rng, Rng};
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use std::{f32::consts::FRAC_PI_2, ops::Div, ops::Mul, ops::Sub};
use wasm_bindgen::prelude::*;

const BOID_SCALE: f32 = 0.28;
const BOID_SPRITE_SCALE: f32 = 6.0;
const BOID_MAX_FORCE: f32 = 10.0;
const BOID_MAX_VELOCITY: f32 = 1.4;
const BOID_MIN_VELOCITY: f32 = 0.8;
const BOID_COHESION: f32 = 40.0;
const BOID_GROUP_SIZE: usize = 20;
const BOID_SEPARATION: f32 = 20.0;
const BOID_SEPARATION_DISTANCE: f32 = 10.0;
const BOID_PERCEPTION: f32 = 30.0;
const BOID_ALIGNMENT: f32 = 10.0;
const BOID_SPEED: f32 = 200.0;
const BOID_ROTATION: f32 = 5.0;
const BOID_WAKE_PER_SECOND: u32 = 20;
const BOID_SPAWN_COUNT: usize = 5;
const WINDOW_BORDER_COLLISION: bool = false;

#[derive(Resource)]
struct BoidCounter {
    pub count: usize,
}

#[inline]
pub fn random_f32() -> f32 {
    thread_rng().gen()
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

    App::new()
        .insert_resource(Msaa::default())
        .add_plugins(
            DefaultPlugins
                .build()
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Bitoids".to_string(),
                        resolution: (1980.0, 1200.0).into(),
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
                            update_subscriber: None,
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, mouse_handler)
        .add_systems(Update, counter_system)
        .add_systems(Update, boid_move_system)
        .add_systems(Update, collision_system)
        .add_systems(Update, boid_acceleration_system.run_if(on_timer(Duration::from_secs_f32(1. / 60.))))
        .run();

    Ok(())
}

// #[derive(Deref)]
// struct BirdTexture(Handle<Image>);

#[derive(Component)]
struct StatsText;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2dBundle::default());

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let ship_atlas = load_ships_atlas(&asset_server, texture_atlases);
    commands.insert_resource(ship_atlas);

    commands.insert_resource(BoidCounter { count: 0 });

    commands
        .spawn(
            TextBundle::from_sections([
                TextSection::new(
                    "Boid Count: ",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 0.0),
                    },
                ),
                TextSection::new(
                    "",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 1.0),
                    },
                ),
                TextSection::new(
                    "\nAverage FPS: ",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 0.0),
                    },
                ),
                TextSection::new(
                    "",
                    TextStyle {
                        font: font.clone(),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 1.0),
                    },
                ),
            ])
            .with_style(Style {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..default()
            }),
        )
        .insert(StatsText);
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
        TextureAtlasLayout::from_grid(Vec2::new(14.0, 14.0), 32, 16, Some(vec2(2.0, 2.0)), None);
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
) {
    let Ok(window) = windows.get_single() else {
        return;
    };

    if mouse_button_input.pressed(MouseButton::Left) {
        spawn_boids(
            &mut commands,
            window,
            &mut counter,
            BOID_SPAWN_COUNT,
            ship_atlas,
        );
    }
}

fn spawn_boids(
    commands: &mut Commands,
    window: &Window,
    counter: &mut BoidCounter,
    spawn_count: usize,
    ship_atlas: Res<ShipAtlas>,
) {
    let mut rng = thread_rng();
    let boid_x = rng.gen::<f32>() * window.width() - window.width() / 2.0;
    let boid_y = rng.gen::<f32>() * window.height() - window.height() / 2.0;

    for count in 0..spawn_count {
        let boid_z = (counter.count + count) as f32 * 0.00001;

        commands
            .spawn(SpriteSheetBundle {
                atlas: TextureAtlas {
                    layout: ship_atlas.layout.clone(),
                    index: rng.gen::<usize>() % (16 * 32),
                },
                texture: ship_atlas.image.clone(),
                sprite: Sprite {
                    anchor: bevy::sprite::Anchor::TopCenter,
                    ..Default::default()
                },
                transform: Transform {
                    translation: Vec3::new(boid_x, boid_y, boid_z),
                    scale: Vec3::splat(BOID_SCALE * BOID_SPRITE_SCALE),
                    ..default()
                },
                ..default()
            })
            .insert(Boid {
                acceleration: vec2(random_f32() - 0.5, random_f32() - 0.5),
                velocity: vec2(
                    rng.gen::<f32>() * BOID_MAX_VELOCITY - (BOID_MAX_VELOCITY * 0.5),
                    rng.gen::<f32>() * BOID_MAX_VELOCITY - (BOID_MAX_VELOCITY * 0.5),
                ),
            });
    }
    counter.count += spawn_count;
}

pub fn collision_system(
    windows: Query<&Window, With<PrimaryWindow>>,
    boid_query: Query<(&mut Boid, &mut Transform)>,
) {
    let Ok(window) = windows.get_single() else {
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
    mut query: Query<&mut Text, With<StatsText>>,
) {
    let mut text = query.single_mut();

    if counter.is_changed() {
        text.sections[1].value = format!("{}", counter.count);
    }

    if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            text.sections[3].value = format!("{:.2}", average);
        }
    };
}

fn boid_move_system(time: Res<Time>, mut query: Query<(Entity, &mut Boid, &mut Transform)>) {
    let delta_speed = time.delta_seconds() * BOID_SPEED;
    for (_, mut boid, mut transform) in query.iter_mut() {
        let acc = boid.acceleration;
        boid.velocity += acc;

        let vel = boid.velocity;
        boid.velocity = set_velocity(BOID_MAX_VELOCITY, BOID_MIN_VELOCITY, &vel);
        transform.translation += vec3(boid.velocity.x, boid.velocity.y, 0.0).mul(delta_speed);

        let angle = { vel.y.atan2(vel.x) + FRAC_PI_2 * 3.0 };
        transform.rotation = transform.rotation.slerp(
            Quat::from_axis_angle(Vec3::new(0., 0., 1.), angle),
            time.delta_seconds() * BOID_ROTATION,
        );
    }
}

pub struct BoidObject {
    pub id: u32,
    pub pos: Vec2,
    pub velocity: Vec2,
}

impl RTreeObject for BoidObject {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        new_point(&self.pos)
    }
}

fn new_point(pos: &Vec2) -> AABB<[f32; 2]> {
    AABB::from_point([pos[0], pos[1]])
}

impl PointDistance for BoidObject {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        self.pos.distance(vec2(point[0], point[1]))
    }

    fn contains_point(&self, point: &[f32; 2]) -> bool {
        let d_x = self.pos[0] - point[0];
        let d_y = self.pos[1] - point[1];
        let distance_to_origin_2 = d_x * d_x + d_y * d_y;
        distance_to_origin_2 <= (BOID_SCALE / 2.0)
    }
}

fn boid_acceleration_system(
    time: Res<Time>,
    mut update_time: Local<Duration>,
    mut group_id: Local<u32>,
    mut query: Query<(Entity, &mut Boid, &mut Transform)>,
) {
    if (time.elapsed() - *update_time).as_secs_f64() < 0.1 / 60.0 {
        return;
    }
    *update_time = time.elapsed();
    *group_id += 1;

    let tree = {
        let boid_array = query
            .iter()
            .map(|(entity, boid, transform)| BoidObject {
                id: entity.index(),
                pos: transform.translation.truncate(),
                velocity: boid.velocity,
            })
            .collect::<Vec<BoidObject>>();
        RTree::bulk_load(boid_array)
    };

    let delta_speed = time.delta_seconds() * BOID_SPEED;
    let gid = *group_id;

    for (entity, mut boid, transform) in query.iter_mut() {
        let entity_id = entity.index();
        if entity_id % BOID_WAKE_PER_SECOND != gid % BOID_WAKE_PER_SECOND {
            continue;
        }

        let pos = transform.translation.truncate();
        let local_boids = tree
            .nearest_neighbor_iter_with_distance_2(&[pos[0], pos[1]])
            .take(BOID_GROUP_SIZE)
            .filter(|(b, v)| b.id != entity_id && *v <= BOID_PERCEPTION)
            .map(|(b, _)| b)
            .collect::<Vec<&BoidObject>>();

        let entity_id = entity.index();
        let alignment = boids_alignment((&boid, &transform), &local_boids);
        let cohesion = boids_cohesion((&boid, &transform), &local_boids);
        let separation = boids_separation((&boid, &transform), &local_boids);
        if entity_id % 30 == 0 {
            info!("entity_id {entity_id} alignment {alignment} cohesion {cohesion} separation {separation} neighbors {neighbors}",
                neighbors = local_boids.len()
            );
        }
        let dir_vel = vec2(0.2, 0.2) * delta_speed;
        if entity_id % 2 == 0 {
            boid.acceleration += (alignment + cohesion + separation + dir_vel).mul(delta_speed);
        } else {
            boid.acceleration += (alignment + cohesion + separation - dir_vel).mul(delta_speed);
        }
        boid.acceleration = set_max_acc(BOID_MAX_FORCE, &boid.acceleration);
    }
}

fn boids_alignment(current_boid: (&Boid, &Transform), local_boids: &[&BoidObject]) -> Vec2 {
    let mut new_velocity = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return new_velocity;
    }

    for boid in local_boids.iter() {
        new_velocity += boid.velocity;
    }
    new_velocity = new_velocity.div(local_boids_len as f32).normalize();
    new_velocity = (new_velocity + current_boid.0.velocity) * BOID_ALIGNMENT;
    new_velocity
}

fn boids_cohesion(current_boid: (&Boid, &Transform), local_boids: &[&BoidObject]) -> Vec2 {
    let mut average_position = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_position;
    }

    for boid in local_boids.iter() {
        average_position += boid.pos
    }
    average_position = average_position.div(local_boids_len as f32);
    average_position = average_position.sub(current_boid.1.translation.truncate()) * BOID_COHESION;
    average_position
}

fn boids_separation(current_boid: (&Boid, &Transform), local_boids: &[&BoidObject]) -> Vec2 {
    let mut average_separation = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_separation;
    }

    for boid in local_boids.iter() {
        let difference_vec = boid.pos.sub(current_boid.1.translation.truncate()).div(
            current_boid
                .1
                .translation
                .distance(vec3(boid.pos[0], boid.pos[1], 0.0))
                * BOID_SEPARATION_DISTANCE,
        );
        average_separation -= difference_vec;
    }

    average_separation * BOID_SEPARATION
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
