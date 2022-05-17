        // mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),

    // for (mut current_boid, transform) in mut_all {
    //     let local_boids = v2
    //         .iter()
    //         .filter(|(_, t)| perception >= transform.translation.distance(t.translation))
    //         .collect::<Vec<&(&Boid, &Transform)>>();

    //     let alignment = boids_alignment((&current_boid, &transform), &local_boids);
    //     let cohesion = boids_cohesion((&current_boid, &transform), &local_boids);
    //     let separation = boids_separation((&current_boid, &transform), &local_boids);
    //     current_boid.acceleration += alignment + cohesion + separation;
    //     current_boid.acceleration = set_max_acc(BIRD_MAX_FORCE, &current_boid.acceleration);
    //     transform.translation += Vec3::new(current_boid.velocity.x, current_boid.velocity.y, 0.0);
    //     let acc = current_boid.acceleration;
    //     current_boid.velocity += acc;
    //     let vel = current_boid.velocity;
    //     current_boid.velocity = set_velocity(BIRD_MAX_VELOCITY, BIRD_MIN_VELOCITY, &vel);

    //     // self.acceleration = vec2(0.0, 0.0);
    //     // transform.translation.x += current_boid.velocity.x * time.delta_seconds();
    //     // transform.translation.y += current_boid.velocity.y * time.delta_seconds();
    //     // boid.velocity.y += GRAVITY * time.delta_seconds();
    // }
