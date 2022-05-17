use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::*,
    prelude::Mut,
    prelude::*,
    render::mesh::{Indices, Mesh, PrimitiveTopology},
    sprite::MaterialMesh2dBundle,
    sprite::Mesh2dHandle,
    window::PresentMode,
};

use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Sub;
use std::rc::Rc;

const BIRDS_PER_SECOND: u32 = 1000; //10000;
const MAX_VELOCITY: f32 = 1750.;
const BIRD_SCALE: f32 = 0.20;
const HALF_BIRD_SIZE: f32 = BIRD_SCALE * 0.5;
const BIRD_MAX_FORCE: f32 = 1.0;
const BIRD_MAX_VELOCITY: f32 = 1.0;
const BIRD_MIN_VELOCITY: f32 = 0.8;
const BIRD_COHESION: f32 = 50.0;
const BIRD_SEPARATION: f32 = 10.0;
const BIRD_PERCEPTION: f32 = 30.0;
const BIRD_ALIGMENT: f32 = 3.5;
const BIRD_SPEED: f32 = 100.0;

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
struct FixedUpdateStage;

struct BevyCounter {
    pub count: usize,
    pub color: Color,
    pub mesh: Mesh2dHandle,
}

#[inline]
pub fn random_f32() -> f32 {
    thread_rng().gen()
}

#[derive(Debug, Component)]
struct Boid {
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub radius: f32,
    // pub max_velocity: f32,
    // pub min_velocity: f32,
    // pub max_force: f32,
}

/// This example provides a 2D benchmark.
///
/// Usage: spawn more entities by clicking on the screen.
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "BevyMark".to_string(),
            width: 1980.,
            height: 1200.,
            present_mode: PresentMode::Immediate,
            resizable: true,
            ..default()
        })
        // .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(mouse_handler)
        .add_system(counter_system)
        .add_system(movement_system)
        .add_system(collision_system)
        .run();
}

#[derive(Deref)]
struct BirdTexture(Handle<Image>);

#[derive(Component)]
struct StatsText;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("icon.png");

    commands.insert_resource(BevyCounter {
        count: 0,
        color: Color::WHITE,
        mesh: meshes.add(create_triangle(HALF_BIRD_SIZE)).into(),
    });

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    commands
        .spawn_bundle(TextBundle {
            text: Text {
                sections: vec![
                    TextSection {
                        value: "Boid Count: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 0.0),
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 1.0),
                        },
                    },
                    TextSection {
                        value: "\nAverage FPS: ".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 40.0,
                            color: Color::rgb(0.0, 1.0, 0.0),
                        },
                    },
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
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

    commands.insert_resource(BirdTexture(texture));
}

fn create_triangle(size: f32) -> Mesh {
    let extent_x = size;
    let extent_y = size;

    let (u_left, u_right) = (0.0, 1.0);
    let vertices = [
        ([-extent_x, -extent_y, 0.0], [0.0, 0.0, 1.0], [u_left, 1.0]),
        ([0.0, extent_y, 0.0], [0.0, 0.0, 1.0], [u_left, 0.0]),
        ([extent_x, -extent_y, 0.0], [0.0, 0.0, 1.0], [u_right, 1.0]),
    ];

    let indices = Indices::U32(vec![0, 2, 1]);

    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    for (position, normal, uv) in &vertices {
        positions.push(*position);
        normals.push(*normal);
        uvs.push(*uv);
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

fn mouse_handler(
    mut commands: Commands,
    time: Res<Time>,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    bird_texture: Res<BirdTexture>,
    mut counter: ResMut<BevyCounter>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        let mut rng = thread_rng();
        counter.color = Color::rgb_linear(rng.gen(), rng.gen(), rng.gen());
    }

    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = (BIRDS_PER_SECOND as f64 * time.delta_seconds_f64()) as usize;
        spawn_birds(
            &mut commands,
            &windows,
            &mut counter,
            spawn_count,
            bird_texture.clone_weak(),
            meshes,
            materials,
        );
    }
}

fn spawn_birds(
    commands: &mut Commands,
    windows: &Windows,
    counter: &mut BevyCounter,
    spawn_count: usize,
    _texture: Handle<Image>,
    _meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let window = windows.primary();
    let mut rng = thread_rng();
    let bird_x = rng.gen::<f32>() * window.width() - window.width() / 2.0;
    let bird_y = rng.gen::<f32>() * window.height() - window.height() / 2.0;

    let material = materials.add(ColorMaterial::from(counter.color));

    for count in 0..spawn_count {
        let bird_z = (counter.count + count) as f32 * 0.00001;

        commands
            // # Mesh triangle
            .spawn_bundle(MaterialMesh2dBundle {
                mesh: counter.mesh.clone(),
                material: material.clone(),
                transform: Transform {
                    translation: vec3(bird_x, bird_y, bird_z),
                    scale: Vec3::splat(64.0),
                    ..default()
                },
                ..Default::default()
            })
            // # Sprite
            // .spawn_bundle(SpriteBundle {
            //     texture: texture.clone(),
            //     transform: Transform {
            //         translation: Vec3::new(bird_x, bird_y, bird_z),
            //         scale: Vec3::splat(BIRD_SCALE),
            //         ..default()
            //     },
            //     sprite: Sprite {
            //         color: counter.color,
            //         ..default()
            //     },
            //     ..default()
            // })
            .insert(Boid {
                acceleration: vec2(random_f32() - 0.5, random_f32() - 0.5),
                velocity: vec2(
                    rng.gen::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                    rng.gen::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                ),
                radius: HALF_BIRD_SIZE,
            });
    }
    counter.count += spawn_count;
}

// fn collision_system(windows: Res<Windows>, mut bird_query: Query<(&mut Boid, &Transform)>) {
//     let window = windows.primary();
//     let half_width = window.width() as f32 * 0.5;
//     let half_height = window.height() as f32 * 0.5;

//     for (mut bird, transform) in bird_query.iter_mut() {
//         let x_vel = bird.velocity.x;
//         let y_vel = bird.velocity.y;
//         let x_pos = transform.translation.x;
//         let y_pos = transform.translation.y;

//         if (x_vel > 0.0 && x_pos + HALF_BIRD_SIZE > half_width)
//             || (x_vel <= 0.0 && x_pos - HALF_BIRD_SIZE < -(half_width))
//         {
//             bird.velocity.x = -x_vel;
//         }
//         if y_vel < 0.0 && y_pos - HALF_BIRD_SIZE < -half_height {
//             bird.velocity.y = -y_vel;
//         }
//         if y_pos + HALF_BIRD_SIZE > half_height && y_vel > 0.0 {
//             bird.velocity.y = 0.0;
//         }
//     }
// }

pub fn collision_system(windows: Res<Windows>, mut bird_query: Query<&mut Transform>) {
    let window = windows.primary();
    let half_width = window.width() as f32 * 0.5;
    let half_height = window.height() as f32 * 0.5;

    for mut transform in bird_query.iter_mut() {
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

fn movement_system(time: Res<Time>, mut query: Query<(Entity, &mut Boid, &mut Transform)>) {
    let boid_array = query
        .iter_mut()
        .map(|x| Rc::new(RefCell::new(x)))
        .collect::<Vec<Rc<RefCell<(Entity, Mut<Boid>, Mut<Transform>)>>>>();

    for node in boid_array.iter() {
        let local_boids = {
            let node = node.borrow();
            let entity_id = node.0.id();

            let mut local_boids = boid_array
                .iter()
                .filter(|n| {
                    entity_id != n.borrow().0.id()
                        && BIRD_PERCEPTION >= n.borrow().2.translation.distance(node.2.translation)
                })
                .collect::<Vec<&Rc<RefCell<(Entity, Mut<Boid>, Mut<Transform>)>>>>();

            local_boids.sort_by(|lhs, rhs| {
                lhs.borrow()
                    .2
                    .translation
                    .distance(node.2.translation)
                    .partial_cmp(&rhs.borrow().2.translation.distance(node.2.translation))
                    .unwrap()
            });

            local_boids
                .iter()
                .take(10)
                .map(|v| v.clone())
                .collect::<Vec<&Rc<RefCell<(Entity, Mut<Boid>, Mut<Transform>)>>>>()
        };

        let dspeed = time.delta_seconds() * BIRD_SPEED;
        let mut node = node.borrow_mut();
        let alignment = boids_alignment((&node.1, &node.2), &local_boids);
        let cohesion = boids_cohesion((&node.1, &node.2), &local_boids);
        let separation = boids_separation((&node.1, &node.2), &local_boids);
        node.1.acceleration += (alignment + cohesion + separation).mul(dspeed);
        node.1.acceleration = set_max_acc(BIRD_MAX_FORCE, &node.1.acceleration);
        let old_x = node.2.translation.x;
        let old_y = node.2.translation.y;
        let x = node.1.velocity.x;
        let y = node.1.velocity.y;
        node.2.translation += vec3(x, y, 0.0).mul(dspeed);
        let acc = node.1.acceleration;
        node.1.velocity += acc;
        let vel = node.1.velocity;
        node.1.velocity = set_velocity(BIRD_MAX_VELOCITY, BIRD_MIN_VELOCITY, &vel);
        node.1.acceleration = vec2(0.0, 0.0);

        node.2.rotation = Quat::from_rotation_z({
            let from_pos = vec2(old_x, old_y);
            let to_pos = node.2.translation.truncate();
            (to_pos - from_pos).angle_between(from_pos)
        });
    }
}

fn boids_alignment<'a>(
    current_boid: (&Boid, &Transform),
    local_boids: &Vec<&Rc<RefCell<(Entity, Mut<Boid>, Mut<Transform>)>>>,
) -> Vec2 {
    let mut average_velocity = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_velocity;
    }

    for node in local_boids.iter() {
        average_velocity += node.borrow().1.velocity;
    }
    average_velocity = average_velocity.div(local_boids_len as f32);
    average_velocity = average_velocity.sub(current_boid.0.velocity) / BIRD_ALIGMENT;
    average_velocity
}

fn boids_cohesion(
    current_boid: (&Boid, &Transform),
    local_boids: &Vec<&Rc<RefCell<(Entity, Mut<Boid>, Mut<Transform>)>>>,
) -> Vec2 {
    let mut average_position = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_position;
    }

    for node in local_boids {
        average_position += node.borrow().2.translation.truncate();
    }
    average_position = average_position.div(local_boids_len as f32);
    average_position = average_position.sub(current_boid.1.translation.truncate()) / BIRD_COHESION;
    average_position
}

fn boids_separation(
    current_boid: (&Boid, &Transform),
    local_boids: &Vec<&Rc<RefCell<(Entity, Mut<Boid>, Mut<Transform>)>>>,
) -> Vec2 {
    let mut average_seperation = vec2(0.0, 0.0);
    let local_boids_len = local_boids.len();
    if local_boids_len == 0 {
        return average_seperation;
    }

    for node in local_boids {
        let t = node.borrow().2.translation;
        let difference_vec = t
            .truncate()
            .sub(current_boid.1.translation.truncate())
            .div(current_boid.1.translation.distance(t) * BIRD_SEPARATION);
        average_seperation -= difference_vec;
    }

    average_seperation * 1.5
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
