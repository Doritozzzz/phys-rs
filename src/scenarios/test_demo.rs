use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::*;
use crate::core::coordinates::{LocalPosition, Sector};
use crate::physics::orbital::vis_viva_velocity;

pub fn spawn_test_demo(world: &mut World) {
    let central_mass = 1e30_f64;

    // Central star at origin
    world.spawn((
        Mass(central_mass),
        Sector(glam::I64Vec3::ZERO),
        LocalPosition(DVec3::ZERO),
        BoundingRadius(50.0),
        Force(DVec3::ZERO),
        Velocity(DVec3::ZERO),
        Acceleration(DVec3::ZERO),
        PreviousAcceleration(DVec3::ZERO),
        BodyType::Star,
        EntityName("Sol".into()),
    ));

    // Three planets in circular orbits
    let planets = [
        (200.0, "Mercurio"),
        (350.0, "Venus"),
        (500.0, "Tierra"),
    ];

    for (i, (radius, name)) in planets.iter().enumerate() {
        let v = vis_viva_velocity(central_mass, *radius, *radius);
        let angle = i as f64 * 2.0 * std::f64::consts::PI / 3.0;
        let pos = DVec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        let vel = DVec3::new(-v * angle.sin(), 0.0, v * angle.cos());

        world.spawn((
            Mass(1e20),
            Sector(glam::I64Vec3::ZERO),
            LocalPosition(pos),
            BoundingRadius(5.0),
            Force(DVec3::ZERO),
            Velocity(vel),
            Acceleration(DVec3::ZERO),
            PreviousAcceleration(DVec3::ZERO),
            BodyType::Planet,
            EntityName(name.to_string()),
        ));
    }
}
