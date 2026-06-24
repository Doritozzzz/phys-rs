use bevy_ecs::prelude::*;
use glam::DVec3;

use crate::components::*;
use crate::components::lensing::GravitationalLens;
use crate::core::coordinates::{LocalPosition, Sector};
use crate::core::time::InitialSnapshot;
use crate::physics::orbital::vis_viva_velocity;

pub fn spawn_lensing_demo(world: &mut World) {
    // ── Black hole: lenses background light ──────────────────────
    let bh_mass = 5e22_f64;
    world.spawn((
        Mass(bh_mass),
        Sector(glam::I64Vec3::ZERO),
        LocalPosition(DVec3::ZERO),
        BoundingRadius(10.0),
        Force(DVec3::ZERO),
        Velocity(DVec3::ZERO),
        Acceleration(DVec3::ZERO),
        PreviousAcceleration(DVec3::ZERO),
        BodyType::BlackHole,
        EntityName("Lens".into()),
        GravitationalLens,
    ));

    // Camera defaults to (0, 10, 30) looking toward origin.
    // Star at z=+80 is "behind" the black hole — its light bends
    // around the black hole to form an Einstein ring.
    let r_behind = 80.0_f64;
    let pos = DVec3::new(0.0, 5.0, r_behind);
    world.spawn((
        Mass(1e30), // Mass is only needed for visual size scaling in the renderer
        Sector(glam::I64Vec3::ZERO),
        LocalPosition(pos),
        BoundingRadius(6.0),
        Temperature(50000.0),
        BodyType::Star,
        EntityName("Supergiant".into()),
    ));

    // ── Foreground stars for visual context ─────────────────────
    let stars = [
        ( 30.0,   0.0, -20.0, 35000.0, "Alpha"),
        (-25.0,  12.0, -30.0, 28000.0, "Beta"),
        ( 15.0, -18.0, -25.0, 40000.0, "Gamma"),
        (-40.0,  -8.0, -50.0, 30000.0, "Delta"),
        (  5.0,  25.0, -15.0, 45000.0, "Epsilon"),
        (-10.0,  -5.0,  50.0, 22000.0, "Zeta"),
    ];

    for (x, y, z, temp, name) in stars {
        let pos = DVec3::new(x, y, z);
        world.spawn((
            Mass(1e29),
            Sector(glam::I64Vec3::ZERO),
            LocalPosition(pos),
            BoundingRadius(3.0),
            Temperature(temp),
            BodyType::Star,
            EntityName(name.to_string()),
        ));
    }

    // ── Snapshot for R key reset ─────────────────────────────────
    let mut snapshot = InitialSnapshot { entities: Vec::new() };
    let mut query = world.query::<(Entity, &Sector, &LocalPosition, Option<&Velocity>)>();
    for (entity, sector, local, velocity) in query.iter(world) {
        let vel = velocity.copied().unwrap_or(Velocity(DVec3::ZERO));
        snapshot.entities.push((entity, *sector, *local, vel));
    }
    world.insert_resource(snapshot);

    println!("→ Lensing demo: black hole at origin + hot stars.");
    println!("  Supergiant at z=+80 km behind BH — Einstein ring visible.");
    println!("  N: toggle lensing | B: bloom | O: orbital cam | click BH: orbit");
}
