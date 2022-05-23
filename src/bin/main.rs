use bevy::{
    core::FixedTimestep,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{Vec2, Vec3, *},
    prelude::*,
    sprite::TextureAtlas,
    window::{PresentMode, WindowMode},
};
use rand::{thread_rng, Rng};
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use std::{f32::consts::FRAC_PI_2, ops::Div, ops::Mul, ops::Sub};

const MAX_VELOCITY: f32 = 2000.;
const BOID_SCALE: f32 = 0.28;
const HALF_BOID_SIZE: f32 = BOID_SCALE * 0.5;
const BOID_SPRITE_SCALE: f32 = 6.0;
const BOID_MAX_FORCE: f32 = 1.0;
const BOID_MAX_VELOCITY: f32 = 1.0;
const BOID_MIN_VELOCITY: f32 = 0.8;
const BOID_COHESION: f32 = 30.0;
const BOID_GROUP_SIZE: usize = 8;
const BOID_SEPARATION: f32 = 2.0;
const BOID_SEPARATION_DISTANCE: f32 = 4.0;
const BOID_PERCEPTION: f32 = 30.0;
const BOID_ALIGMENT: f32 = 2.0;
const BOID_SPEED: f32 = 100.0;
const BOID_ROTATION: f32 = 4.0;
const BOID_WAKE_PER_SECOND: u32 = 5;
const WINDOR_BORDER_COLLISION: bool = false;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

struct BevyCounter {
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

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "BevyMark".to_string(),
            width: 1980.,
            height: 1200.,
            mode: WindowMode::BorderlessFullscreen,
            present_mode: PresentMode::Immediate,
            resizable: true,
            position: Some(vec2(0.0, 0.0)),
            fit_canvas_to_parent: true,
            // canvas: Some("#canvas".to_string()),
            ..default()
        })
        // .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(mouse_handler)
        .add_system(counter_system)
        .add_system(boid_move_system)
        .add_system(collision_system)
        .add_stage_after(
            CoreStage::Update,
            FixedUpdateStage,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(1f64 / 60f64))
                .with_system(boid_acceleration_system),
        )
        .run();
}

#[derive(Deref)]
struct BirdTexture(Handle<Image>);

#[derive(Component)]
struct StatsText;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let ship_atlas = load_ships_atlas(&asset_server, texture_atlases);
    commands.insert_resource(ship_atlas);

    commands.insert_resource(BevyCounter { count: 0 });

    commands
        .spawn_bundle(TextBundle {
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Boid Count: ".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 0.0),
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 1.0),
                        },
                    },
                    TextSection {
                        value: "\nAverage FPS: ".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 0.0),
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 1.0),
                        },
                    },
                ],
                ..default()
            },
            style: Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..default()
                },
                ..default()
            },
            ..default()
        })
        .insert(StatsText);
}

#[derive(Deref)]
struct ShipAtlas(Handle<TextureAtlas>);

fn load_ships_atlas(
    asset_server: &AssetServer,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) -> ShipAtlas {
    let texture_handle = asset_server.load("ships001.png");
    let texture_atlas = TextureAtlas::from_grid_with_padding(
        texture_handle,
        Vec2::new(14.0, 14.0),
        16,
        32,
        vec2(2.0, 2.0),
    );
    let atlas_handle = texture_atlases.add(texture_atlas);
    ShipAtlas(atlas_handle)
}

fn mouse_handler(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    mut counter: ResMut<BevyCounter>,
    ship_atlas: Res<ShipAtlas>,
) {
    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = 6;
        spawn_boids(
            &mut commands,
            &windows,
            &mut counter,
            spawn_count,
            ship_atlas,
        );
    }
}

fn spawn_boids(
    commands: &mut Commands,
    windows: &Windows,
    counter: &mut BevyCounter,
    spawn_count: usize,
    ship_atlas: Res<ShipAtlas>,
) {
    let window = windows.primary();
    let mut rng = thread_rng();
    let boid_x = rng.gen::<f32>() * window.width() - window.width() / 2.0;
    let boid_y = rng.gen::<f32>() * window.height() - window.height() / 2.0;

    for count in 0..spawn_count {
        let boid_z = (counter.count + count) as f32 * 0.00001;

        commands
            .spawn_bundle(SpriteSheetBundle {
                texture_atlas: ship_atlas.clone(),
                sprite: TextureAtlasSprite {
                    index: rng.gen::<usize>() % (16 * 32),
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
                    rng.gen::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                    rng.gen::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                ),
            });
    }
    counter.count += spawn_count;
}

pub fn collision_system(windows: Res<Windows>, boid_query: Query<(&mut Boid, &mut Transform)>) {
    if WINDOR_BORDER_COLLISION {
        window_bounce_collision_system(windows, boid_query);
    } else {
        window_teleport_collision_system(windows, boid_query);
    }
}

fn window_bounce_collision_system(
    windows: Res<Windows>,
    mut boid_query: Query<(&mut Boid, &mut Transform)>,
) {
    let window = windows.primary();
    let half_width = window.width() as f32 * 0.5;
    let half_height = window.height() as f32 * 0.5;

    for (mut boid, transform) in boid_query.iter_mut() {
        let x_vel = boid.velocity.x;
        let y_vel = boid.velocity.y;
        let x_pos = transform.translation.x;
        let y_pos = transform.translation.y;

        if (x_vel > 0.0 && x_pos + HALF_BOID_SIZE > half_width)
            || (x_vel <= 0.0 && x_pos - HALF_BOID_SIZE < -(half_width))
        {
            boid.velocity.x = -x_vel;
        }
        if y_vel < 0.0 && y_pos - HALF_BOID_SIZE < -half_height {
            boid.velocity.y = -y_vel;
        }
        if y_pos + HALF_BOID_SIZE > half_height && y_vel > 0.0 {
            boid.velocity.y = 0.0;
        }
    }
}

fn window_teleport_collision_system(
    windows: Res<Windows>,
    mut boid_query: Query<(&mut Boid, &mut Transform)>,
) {
    let window = windows.primary();
    let half_width = window.width() as f32 * 0.5;
    let half_height = window.height() as f32 * 0.5;

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
    diagnostics: Res<Diagnostics>,
    counter: Res<BevyCounter>,
    mut query: Query<&mut Text, With<StatsText>>,
) {
    let mut text = query.single_mut();

    if counter.is_changed() {
        text.sections[1].value = format!("{}", counter.count);
    }

    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            text.sections[3].value = format!("{:.2}", average);
        }
    };
}

fn boid_move_system(time: Res<Time>, mut query: Query<(Entity, &mut Boid, &mut Transform)>) {
    let dspeed = time.delta_seconds() * BOID_SPEED;
    for (_, mut boid, mut transform) in query.iter_mut() {
        let acc = boid.acceleration;
        boid.velocity += acc;

        let vel = boid.velocity;
        boid.velocity = set_velocity(BOID_MAX_VELOCITY, BOID_MIN_VELOCITY, &vel);
        transform.translation += vec3(boid.velocity.x, boid.velocity.y, 0.0).mul(dspeed);

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
        distance_to_origin_2 <= HALF_BOID_SIZE
    }
}

fn boid_acceleration_system(
    time: Res<Time>,
    mut update_time: Local<f64>,
    mut group_id: Local<u32>,
    mut query: Query<(Entity, &mut Boid, &mut Transform)>,
) {
    if time.seconds_since_startup() - *update_time < 0.1 / 60.0 {
        return;
    }
    *update_time = time.seconds_since_startup();
    *group_id = *group_id + 1;

    let tree = {
        let boid_array = query
            .iter()
            .map(|(entity, boid, transform)| BoidObject {
                id: entity.id(),
                pos: transform.translation.truncate(),
                velocity: boid.velocity,
            })
            .collect::<Vec<BoidObject>>();
        RTree::bulk_load(boid_array)
    };

    let dspeed = time.delta_seconds() * BOID_SPEED;
    let gid = *group_id;

    for (entity, mut boid, transform) in query.iter_mut() {
        let entity_id = entity.id();
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

        let entity_id = entity.id();
        let alignment = boids_alignment((&boid, &transform), &local_boids);
        let cohesion = boids_cohesion((&boid, &transform), &local_boids);
        let separation = boids_separation((&boid, &transform), &local_boids);
        if entity_id % 2 == 0 {
            boid.acceleration +=
                (alignment + cohesion + separation + vec2(-0.002, 0.002)).mul(dspeed);
        } else {
            boid.acceleration +=
                (alignment + cohesion + separation + vec2(0.002, 0.002)).mul(dspeed);
        }
        boid.acceleration = set_max_acc(BOID_MAX_FORCE, &boid.acceleration);
    }
}

fn boids_alignment<'a>(current_boid: (&Boid, &Transform), local_boids: &Vec<&BoidObject>) -> Vec2 {
    let mut average_velocity = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_velocity;
    }

    for boid in local_boids.into_iter() {
        average_velocity += boid.velocity;
    }
    average_velocity = average_velocity.div(local_boids_len as f32);
    average_velocity = average_velocity.sub(current_boid.0.velocity) / BOID_ALIGMENT;
    average_velocity
}

fn boids_cohesion(current_boid: (&Boid, &Transform), local_boids: &Vec<&BoidObject>) -> Vec2 {
    let mut average_position = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_position;
    }

    for boid in local_boids.into_iter() {
        average_position += boid.pos
    }
    average_position = average_position.div(local_boids_len as f32);
    average_position = average_position.sub(current_boid.1.translation.truncate()) / BOID_COHESION;
    average_position
}

fn boids_separation(current_boid: (&Boid, &Transform), local_boids: &Vec<&BoidObject>) -> Vec2 {
    let mut average_seperation = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_seperation;
    }

    for boid in local_boids.into_iter() {
        let difference_vec = boid.pos.sub(current_boid.1.translation.truncate()).div(
            current_boid
                .1
                .translation
                .distance(vec3(boid.pos[0], boid.pos[1], 0.0))
                * BOID_SEPARATION_DISTANCE,
        );
        average_seperation -= difference_vec;
    }

    average_seperation * BOID_SEPARATION
}

fn set_max_acc(max_acc: f32, acc: &Vec2) -> Vec2 {
    let acc_len = acc.length_squared();

    let mut new_acc = acc.clone();

    if acc_len > max_acc * max_acc {
        new_acc = acc.normalize_or_zero();
        new_acc = new_acc.mul(max_acc);
    }
    new_acc
}

fn set_velocity(max_vel: f32, min_vel: f32, vel: &Vec2) -> Vec2 {
    let vel_len = vel.length_squared();

    let mut new_vel = vel.clone();

    if vel_len > max_vel * max_vel {
        new_vel = vel.normalize_or_zero();
        new_vel = new_vel.mul(max_vel);
    } else if vel_len < min_vel * min_vel {
        new_vel = vel.normalize_or_zero();
        new_vel = new_vel.mul(min_vel);
    }
    new_vel
}
