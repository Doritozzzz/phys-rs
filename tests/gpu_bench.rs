//! Phase 12.9 — Benchmark suite for CPU/GPU performance comparison.
//!
//! Run with: cargo test --test gpu_bench -- --ignored --nocapture
//!
//! These benchmarks measure:
//! 1. Gravity CPU single-thread vs Rayon vs Barnes-Hut
//! 2. SPH CPU pipeline throughput at various particle counts
//! 3. GPU dispatch overhead (if GPU available)
//! 4. Overall throughput (particles/second)

use std::time::Instant;

// ─── Helpers ───────────────────────────────────────────────────────

/// Format a duration in human-readable form.
fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs >= 1.0 {
        format!("{:.3}s", secs)
    } else if secs >= 1e-3 {
        format!("{:.1}ms", secs * 1e3)
    } else if secs >= 1e-6 {
        format!("{:.1}µs", secs * 1e6)
    } else {
        format!("{:.1}ns", secs * 1e9)
    }
}

/// Create a world populated with N gravity bodies.
fn setup_gravity_world(n: usize) -> bevy_ecs::world::World {
    use bevy_ecs::prelude::*;
    use glam::DVec3;
    use phys_rs::components::dynamics::{Force, Mass};
    use phys_rs::components::spatial::BoundingRadius;
    use phys_rs::core::config::UniverseConfig;
    use phys_rs::core::coordinates::{LocalPosition, Sector};

    let mut world = World::new();
    let mut config = UniverseConfig::default();
    config.softening_epsilon = 1e-4;
    world.insert_resource(config);

    // Random cluster
    let sigma = 100.0_f64;
    for i in 0..n {
        use std::f64::consts::PI;
        let theta = (i as f64) * 2.0 * PI * 0.618033988749895; // golden angle
        let phi = ((i as f64) * PI * 0.618033988749895).cos().acos();
        let r = sigma * (i as f64).powf(0.333);
        let pos = DVec3::new(
            r * phi.sin() * theta.cos(),
            r * phi.sin() * theta.sin(),
            r * phi.cos(),
        );
        world.spawn((
            Force(DVec3::ZERO),
            Mass(1e10_f64),
            Sector::default(),
            LocalPosition(pos),
            BoundingRadius(0.0),
        ));
    }
    world
}

/// Create a world populated with N fluid SPH particles.
fn setup_sph_world(n: usize) -> bevy_ecs::world::World {
    use bevy_ecs::prelude::*;
    use glam::DVec3;
    use phys_rs::components::dynamics::{Acceleration, Force, Mass, Velocity};
    use phys_rs::components::sph::{FluidParticle, Pressure, SmoothedDensity, SmoothingRadius};
    use phys_rs::core::config::UniverseConfig;
    use phys_rs::core::coordinates::{LocalPosition, Sector};
    use phys_rs::physics::broadphase::{CandidatePair, CandidatePairs, KdTreeBroadphase};

    let mut world = World::new();
    let mut config = UniverseConfig::default();
    config.sector_size = 1e6_f64;
    world.insert_resource(config);
    world.insert_resource(phys_rs::physics::sph::TaitEquationConfig::default());
    world.insert_resource(phys_rs::physics::sph::SphViscosityConfig::default());
    world.insert_resource(phys_rs::physics::sph::XsphConfig::default());
    world.insert_resource(phys_rs::physics::sph::SphBoundaryConfig::default());

    // Cubic lattice of fluid particles
    let spacing = 0.05_f64;
    let h = 0.12_f64;
    let mass = 50.0_f64;
    let ppd = (n as f64).powf(1.0 / 3.0).ceil() as usize;
    let mut ents = Vec::new();
    for ix in 0..ppd {
        for iy in 0..ppd {
            for iz in 0..ppd {
                if ents.len() >= n { break; }
                let pos = DVec3::new(
                    ix as f64 * spacing,
                    iy as f64 * spacing,
                    iz as f64 * spacing,
                );
                ents.push(world.spawn((
                    Sector::default(), LocalPosition(pos),
                    Mass(mass), Velocity(DVec3::ZERO), Force(DVec3::ZERO),
                    Acceleration(DVec3::ZERO), SmoothedDensity(0.0),
                    Pressure(0.0), SmoothingRadius(h), FluidParticle,
                )).id());
            }
            if ents.len() >= n { break; }
        }
        if ents.len() >= n { break; }
    }

    // Build candidate pairs (brute-force for benchmark)
    let positions: Vec<DVec3> = ents.iter()
        .map(|e| world.get::<LocalPosition>(*e).unwrap().0)
        .collect();
    let mut pairs = Vec::new();
    for i in 0..ents.len() {
        for j in (i + 1)..ents.len() {
            let d = (positions[i] - positions[j]).length();
            if d < 2.0 * h {
                pairs.push(CandidatePair { entity_a: ents[i], entity_b: ents[j] });
            }
        }
    }
    world.insert_resource(CandidatePairs(pairs));
    world.insert_resource(KdTreeBroadphase::default());
    world.insert_resource(phys_rs::gpu::SphMode::Cpu);
    world.insert_resource(phys_rs::core::time::SimulationTime::with_dt(1e-4));

    world
}

// ─── Benchmark 1: Gravity CPU vs Rayon vs Barnes-Hut ──────────────

#[test]
#[ignore]
fn bench_gravity_1000() {
    bench_gravity("N=1000", 1000);
}

#[test]
#[ignore]
fn bench_gravity_5000() {
    bench_gravity("N=5000", 5000);
}

#[test]
#[ignore]
fn bench_gravity_10000() {
    bench_gravity("N=10000", 10000);
}

fn bench_gravity(label: &str, n: usize) {
    use bevy_ecs::prelude::*;
    use phys_rs::physics::gravity::brute_force_gravity_system;
    use phys_rs::physics::gravity_tree::barnes_hut_gravity_system;

    let mut world = setup_gravity_world(n);

    // Brute force (Rayon — already parallel)
    let mut sched = Schedule::default();
    sched.add_systems(brute_force_gravity_system);
    let start = Instant::now();
    for _ in 0..10 {
        // Reset forces (need a separate schedule for this since we can't borrow world mutably twice)
        {
            let mut query = world.query::<&mut phys_rs::components::dynamics::Force>();
            for mut f in query.iter_mut(&mut world) {
                f.0 = glam::DVec3::ZERO;
            }
        }
        sched.run(&mut world);
    }
    let bf_dur = start.elapsed() / 10;
    let bf_throughput = (n as f64) / bf_dur.as_secs_f64();

    // Barnes-Hut
    let mut sched_bh = Schedule::default();
    sched_bh.add_systems(barnes_hut_gravity_system);
    let start = Instant::now();
    for _ in 0..10 {
        {
            let mut query = world.query::<&mut phys_rs::components::dynamics::Force>();
            for mut f in query.iter_mut(&mut world) {
                f.0 = glam::DVec3::ZERO;
            }
        }
        sched_bh.run(&mut world);
    }
    let bh_dur = start.elapsed() / 10;
    let bh_throughput = (n as f64) / bh_dur.as_secs_f64();

    println!("[{label}] Brute-Force (Rayon): {} avg, {:.0} particles/s",
        format_duration(bf_dur), bf_throughput);
    println!("[{label}] Barnes-Hut (θ=0.5):  {} avg, {:.0} particles/s",
        format_duration(bh_dur), bh_throughput);
    println!("[{label}] Speedup BH/BF: {:.2}x", bf_dur.as_secs_f64() / bh_dur.as_secs_f64());
}

// ─── Benchmark 2: SPH CPU pipeline ────────────────────────────────

#[test]
#[ignore]
fn bench_sph_cpu_100() {
    bench_sph_cpu("SPH N=100", 100);
}

#[test]
#[ignore]
fn bench_sph_cpu_500() {
    bench_sph_cpu("SPH N=500", 500);
}

#[test]
#[ignore]
fn bench_sph_cpu_1000() {
    bench_sph_cpu("SPH N=1000", 1000);
}

fn bench_sph_cpu(label: &str, n: usize) {
    use bevy_ecs::prelude::*;
    use phys_rs::physics::sph::{
        sph_density_system, sph_eos_system, sph_pressure_force_system, sph_viscosity_system,
        sph_xsph_system, sph_boundary_system,
    };

    let mut world = setup_sph_world(n);

    // Full SPH pipeline
    let mut sched = Schedule::default();
    sched.add_systems((
        sph_density_system,
        sph_eos_system.after(sph_density_system),
        sph_pressure_force_system.after(sph_eos_system),
        sph_viscosity_system.after(sph_pressure_force_system),
    ).chain());
    sched.add_systems((
        sph_xsph_system,
        sph_boundary_system,
    ));

    // Warm-up
    sched.run(&mut world);

    let start = Instant::now();
    for _ in 0..20 {
        sched.run(&mut world);
    }
    let dur = start.elapsed() / 20;
    let throughput = (n as f64) / dur.as_secs_f64();

    println!("[{label}] CPU SPH pipeline: {} avg, {:.0} particles/s",
        format_duration(dur), throughput);
}

// ─── Benchmark 3: Gravity scaling comparison ───────────────────────

#[test]
#[ignore]
fn bench_gravity_scaling() {
    let sizes = [100, 500, 1000, 5000];
    println!("\n── Gravity Scaling ──");
    for n in &sizes {
        bench_gravity(&format!("N={}", n), *n);
    }
}

// ─── Benchmark 4: SPH scaling ─────────────────────────────────────

#[test]
#[ignore]
fn bench_sph_scaling() {
    let sizes = [64, 125, 216, 512, 1000];
    println!("\n── SPH CPU Scaling ──");
    for n in &sizes {
        bench_sph_cpu(&format!("N={}", n), *n);
    }
}
