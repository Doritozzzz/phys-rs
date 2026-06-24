use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::*;
use crate::core::coordinates::{LocalPosition, Sector};
use crate::core::time::InitialSnapshot;
use crate::physics::orbital::vis_viva_velocity;

pub fn spawn_test_demo(world: &mut World) {
    let central_mass = 1e22_f64;

    let star = world.spawn((
        Mass(central_mass),
        Sector(glam::I64Vec3::ZERO),
        LocalPosition(DVec3::ZERO),
        BoundingRadius(10.0),
        Force(DVec3::ZERO),
        Velocity(DVec3::ZERO),
        Acceleration(DVec3::ZERO),
        PreviousAcceleration(DVec3::ZERO),
        BodyType::Star,
        EntityName("Sol".into()),
    ));

    // Three planets in circular orbits (periods ~40-130s with M=1e22)
    let planets = [
        (30_000.0_f64, "Mercurio"),
        (45_000.0_f64, "Venus"),
        (65_000.0_f64, "Tierra"),
    ];

    for (i, (radius, name)) in planets.iter().enumerate() {
        let v = vis_viva_velocity(central_mass, *radius, *radius);
        let angle = i as f64 * 2.0 * std::f64::consts::PI / 3.0;
        let pos = DVec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        let vel = DVec3::new(-v * angle.sin(), 0.0, v * angle.cos());

        world.spawn((
            Mass(1e18),
            Sector(glam::I64Vec3::ZERO),
            LocalPosition(pos),
            BoundingRadius(2.0),
            Force(DVec3::ZERO),
            Velocity(vel),
            Acceleration(DVec3::ZERO),
            PreviousAcceleration(DVec3::ZERO),
            BodyType::Planet,
            EntityName(name.to_string()),
            OrbitTrail::default(),
        ));
    }

    // Snapshot for R key reset
    let mut snapshot = InitialSnapshot { entities: Vec::new() };
    let mut query = world.query::<(Entity, &Sector, &LocalPosition, &Velocity)>();
    for (entity, sector, local, velocity) in query.iter(world) {
        snapshot.entities.push((entity, *sector, *local, *velocity));
    }
    world.insert_resource(snapshot);
}
