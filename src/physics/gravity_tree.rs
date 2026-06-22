//! Barnes-Hut octree gravity approximation (PHYSICS_MASTER_INDEX §IV.1).
//!
//! Provides O(N log N) gravitational force computation by grouping
//! distant bodies into aggregate mass nodes. The opening angle θ
//! from [`UniverseConfig::barnes_hut_theta`] controls the
//! accuracy/performance tradeoff.
//!
//! ## Algorithm
//! 1. Flatten all `(Sector, LocalPosition)` into absolute `DVec3` positions
//!    relative to a chosen origin.
//! 2. Build an octree by recursive spatial subdivision.
//! 3. For each body, walk the tree: if `s / d < θ` (node size / distance),
//!    treat the node as a single point mass at its center of mass.
//!    Otherwise, recurse into children.
//! 4. Accumulate forces using the same softened gravitational law as
//!    [`brute_force_gravity_system`](super::gravity::brute_force_gravity_system).

use bevy_ecs::prelude::*;
use glam::DVec3;
use rayon::prelude::*;

use crate::components::dynamics::{Force, Mass};
use crate::core::config::UniverseConfig;
use crate::core::coordinates::{LocalPosition, Sector};

// ─── Octree Data Structure ─────────────────────────────────────────

/// Maximum depth of the octree. Prevents infinite recursion when
/// two particles occupy the exact same position.
const MAX_DEPTH: u32 = 40;

/// Minimum node size [m]. Nodes smaller than this are treated as
/// leaves regardless of particle count.
const MIN_NODE_SIZE: f64 = 1e-8_f64;

/// A body's data flattened into absolute coordinates for tree insertion.
#[derive(Clone, Copy)]
struct BodyData {
    /// Absolute position in the common frame [m].
    pos: DVec3,
    /// Mass [kg].
    mass: f64,
    /// Index into the original entity list (for writing forces back).
    index: usize,
}

/// An octree node for the Barnes-Hut algorithm.
///
/// Each node represents a cubic region of space and stores:
/// - The aggregate mass and center of mass of all bodies inside it.
/// - Either a single body (leaf) or eight children (internal node).
struct OctreeNode {
    /// Center of this cubic node [m].
    center: DVec3,
    /// Half-side-length of this cubic node [m].
    half_size: f64,
    /// Total mass of all bodies in this subtree [kg].
    total_mass: f64,
    /// Center of mass of all bodies in this subtree [m].
    center_of_mass: DVec3,
    /// Node content: either empty, a single body, or eight children.
    content: NodeContent,
}

enum NodeContent {
    /// No bodies in this region.
    Empty,
    /// Exactly one body (leaf node).
    Leaf(BodyData),
    /// Eight children (internal node), indexed by octant.
    /// Stored on the heap to keep `OctreeNode` small on the stack.
    Internal(Box<[OctreeNode; 8]>),
}

impl OctreeNode {
    /// Create an empty node covering the given cubic region.
    fn new(center: DVec3, half_size: f64) -> Self {
        Self {
            center,
            half_size,
            total_mass: 0.0_f64,
            center_of_mass: DVec3::ZERO,
            content: NodeContent::Empty,
        }
    }

    /// Determine which octant (0–7) a position falls into relative
    /// to this node's center.
    #[inline]
    fn octant_index(&self, pos: DVec3) -> usize {
        let mut idx = 0_usize;
        if pos.x >= self.center.x { idx |= 1; }
        if pos.y >= self.center.y { idx |= 2; }
        if pos.z >= self.center.z { idx |= 4; }
        idx
    }

    /// Compute the center of the child node at the given octant index.
    #[inline]
    fn child_center(&self, octant: usize) -> DVec3 {
        let q = self.half_size * 0.5_f64;
        DVec3::new(
            self.center.x + if octant & 1 != 0 { q } else { -q },
            self.center.y + if octant & 2 != 0 { q } else { -q },
            self.center.z + if octant & 4 != 0 { q } else { -q },
        )
    }

    /// Insert a body into this node, subdividing as necessary.
    fn insert(&mut self, body: BodyData, depth: u32) {
        // Update aggregate mass and center of mass incrementally:
        // COM = (old_mass * old_COM + new_mass * new_pos) / total_mass
        let new_total = self.total_mass + body.mass;
        if new_total > 0.0_f64 {
            self.center_of_mass =
                (self.center_of_mass * self.total_mass + body.pos * body.mass) / new_total;
        }
        self.total_mass = new_total;

        // Safety: prevent infinite recursion
        if depth >= MAX_DEPTH || self.half_size < MIN_NODE_SIZE {
            // Force this to be a leaf (may lose accuracy for co-located particles,
            // but prevents stack overflow).
            self.content = NodeContent::Leaf(body);
            return;
        }

        match std::mem::replace(&mut self.content, NodeContent::Empty) {
            NodeContent::Empty => {
                // This node was empty — just store the body as a leaf.
                self.content = NodeContent::Leaf(body);
            }
            NodeContent::Leaf(existing) => {
                // This was a leaf with one body. We need to subdivide:
                // create 8 children and re-insert both the existing body
                // and the new body.
                let hs = self.half_size * 0.5_f64;
                let children = Box::new(std::array::from_fn(|i| {
                    OctreeNode::new(self.child_center(i), hs)
                }));
                self.content = NodeContent::Internal(children);

                // Re-insert the existing body
                self.insert_into_children(existing, depth + 1);
                // Insert the new body
                self.insert_into_children(body, depth + 1);
            }
            NodeContent::Internal(children) => {
                // Already subdivided — restore children and insert.
                self.content = NodeContent::Internal(children);
                self.insert_into_children(body, depth + 1);
            }
        }
    }

    /// Insert a body into the appropriate child node.
    fn insert_into_children(&mut self, body: BodyData, depth: u32) {
        let octant = self.octant_index(body.pos);
        if let NodeContent::Internal(ref mut children) = self.content {
            children[octant].insert(body, depth);
        }
    }

    /// Compute the gravitational force on `target` from this node.
    ///
    /// Uses the Barnes-Hut opening criterion: if the node's side length
    /// divided by the distance to its center of mass is less than θ,
    /// the entire node is treated as a single point mass.
    ///
    /// Returns the accumulated force vector [N].
    fn compute_force(
        &self,
        target: &BodyData,
        theta: f64,
        g: f64,
        eps2: f64,
    ) -> DVec3 {
        match &self.content {
            NodeContent::Empty => DVec3::ZERO,
            NodeContent::Leaf(body) => {
                if body.index == target.index {
                    // Don't compute self-interaction.
                    return DVec3::ZERO;
                }
                // Direct particle-particle force
                compute_gravity_pair(target, body.pos, body.mass, g, eps2)
            }
            NodeContent::Internal(children) => {
                // Opening criterion: s / d < θ
                let r_vec = self.center_of_mass - target.pos;
                let d2 = r_vec.length_squared();
                let s = self.half_size * 2.0_f64; // full side length

                if s * s < theta * theta * d2 {
                    // Node is far enough → treat as a single body
                    compute_gravity_pair(target, self.center_of_mass, self.total_mass, g, eps2)
                } else {
                    // Node is too close → recurse into children
                    let mut f = DVec3::ZERO;
                    for child in children.iter() {
                        if child.total_mass > 0.0_f64 {
                            f += child.compute_force(target, theta, g, eps2);
                        }
                    }
                    f
                }
            }
        }
    }
}

/// Compute the softened gravitational force on `target` from a point
/// mass at `source_pos` with `source_mass`.
///
/// Uses the same formula as `brute_force_gravity_system`:
/// `F⃗ = G * m_target * m_source * r⃗ / (r² + ε²)^(3/2)`
#[inline]
fn compute_gravity_pair(
    target: &BodyData,
    source_pos: DVec3,
    source_mass: f64,
    g: f64,
    eps2: f64,
) -> DVec3 {
    let r_vec = source_pos - target.pos;
    let r2 = r_vec.length_squared();
    let softened_r2 = r2 + eps2;
    let softened_r = softened_r2.sqrt();
    let softened_r3 = softened_r2 * softened_r;

    // (G * m_target) * m_source / r³ * r_vec
    let scalar_f = ((g * target.mass) * source_mass) / softened_r3;
    r_vec * scalar_f
}

// ─── ECS System ────────────────────────────────────────────────────

/// Barnes-Hut octree gravity system (PHYSICS_MASTER_INDEX §IV.1).
///
/// O(N log N) approximation of N-body gravitational interactions.
///
/// ## Algorithm
/// 1. Collects all massive bodies and flattens their sector-based
///    positions into absolute `DVec3` coordinates.
/// 2. Builds an octree covering all bodies with a 10% margin.
/// 3. For each body, traverses the tree using the opening angle
///    `θ = config.barnes_hut_theta` to decide when to approximate
///    distant clusters as point masses.
/// 4. Writes the resulting force back into each body's `Force` component.
///
/// ## Accuracy
/// - θ = 0.0: Exact (degenerates to O(N²) brute force).
/// - θ = 0.5: Good balance of accuracy and speed (default).
/// - θ = 1.0: Fast but less accurate; suitable for visualization.
///
/// ## Numerical Safety
/// - Uses the same softening `ε²` as `brute_force_gravity_system`.
/// - All arithmetic in `f64` / `DVec3`.
/// - `debug_assert!` on finiteness of final accumulated force.
pub fn barnes_hut_gravity_system(
    mut query: Query<(Entity, &mut Force, &Mass, &Sector, &LocalPosition)>,
    config: Res<UniverseConfig>,
) {
    let g = config.gravitational_constant;
    let eps2 = config.softening_epsilon * config.softening_epsilon;
    let theta = config.barnes_hut_theta;
    let sector_size = config.sector_size;

    // ── Step 1: Flatten positions ──────────────────────────────────
    // Convert (Sector, LocalPosition) → absolute DVec3 relative to
    // the first body's sector to keep coordinates moderate.
    let mut bodies: Vec<BodyData> = Vec::new();
    let mut entities: Vec<Entity> = Vec::new();

    // Pick an origin sector to keep absolute coordinates reasonable.
    // Using the first entity's sector prevents large-coordinate drift.
    let mut origin_sector = Sector::ORIGIN;
    let mut first = true;

    for (entity, _, mass, sector, local) in query.iter() {
        if first {
            origin_sector = *sector;
            first = true; // only set once
        }

        let sector_delta = sector.0 - origin_sector.0;
        let abs_pos = DVec3::new(
            sector_delta.x as f64 * sector_size + local.0.x,
            sector_delta.y as f64 * sector_size + local.0.y,
            sector_delta.z as f64 * sector_size + local.0.z,
        );

        bodies.push(BodyData {
            pos: abs_pos,
            mass: mass.0,
            index: bodies.len(),
        });
        entities.push(entity);
    }

    if bodies.len() < 2 {
        return; // No pairs to compute
    }

    // ── Step 2: Compute bounding box ──────────────────────────────
    let mut min_corner = bodies[0].pos;
    let mut max_corner = bodies[0].pos;
    for body in &bodies {
        min_corner = min_corner.min(body.pos);
        max_corner = max_corner.max(body.pos);
    }

    let extent = max_corner - min_corner;
    let max_extent = extent.x.max(extent.y).max(extent.z);
    // Add 10% margin so no body sits exactly on the boundary
    let half_size = max_extent * 0.55_f64 + 1.0_f64; // +1m for degenerate cases
    let center = (min_corner + max_corner) * 0.5_f64;

    // ── Step 3: Build octree ──────────────────────────────────────
    let mut root = OctreeNode::new(center, half_size);
    for body in &bodies {
        root.insert(*body, 0);
    }

    // ── Step 4: Compute forces ────────────────────────────────────
    let forces: Vec<DVec3> = bodies.par_iter().with_min_len(1024).map(|body| {
        let f = root.compute_force(body, theta, g, eps2);
        debug_assert!(f.is_finite(), "NaN/Inf in Barnes-Hut force for body {}", body.index);
        f
    }).collect();

    // ── Step 5: Write forces back to ECS ──────────────────────────
    for (i, entity) in entities.iter().enumerate() {
        if let Ok((_, mut force, _, _, _)) = query.get_mut(*entity) {
            force.0 += forces[i];
        }
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::dynamics::Force;
    use crate::core::coordinates::Sector;

    /// Two-body test: Barnes-Hut should produce the same force as
    /// brute-force when there are only two bodies (no approximation).
    #[test]
    fn test_barnes_hut_matches_brute_force_two_bodies() {
        let mut world = World::new();

        let config = UniverseConfig {
            gravitational_constant: 6.67430e-11_f64,
            softening_epsilon: 1e-4_f64,
            barnes_hut_theta: 0.5_f64,
            sector_size: 1e12_f64,
            ..UniverseConfig::default()
        };
        world.insert_resource(config.clone());

        let m1 = 1e10_f64;
        let m2 = 2e10_f64;
        let distance = 100.0_f64;

        let e1 = world.spawn((
            Force(DVec3::ZERO),
            Mass(m1),
            Sector::ORIGIN,
            LocalPosition(DVec3::ZERO),
        )).id();

        let e2 = world.spawn((
            Force(DVec3::ZERO),
            Mass(m2),
            Sector::ORIGIN,
            LocalPosition(DVec3::new(distance, 0.0, 0.0)),
        )).id();

        // Run Barnes-Hut
        let mut schedule = Schedule::default();
        schedule.add_systems(barnes_hut_gravity_system);
        schedule.run(&mut world);

        let f1 = world.get::<Force>(e1).unwrap().0;
        let f2 = world.get::<Force>(e2).unwrap().0;

        // Analytical force x-component:
        // F_x = G * m1 * m2 * distance / (r² + ε²)^(3/2)
        let eps2 = config.softening_epsilon * config.softening_epsilon;
        let r2 = distance * distance;
        let softened_r3 = (r2 + eps2).sqrt() * (r2 + eps2);
        let expected_fx = config.gravitational_constant * m1 * m2 * distance / softened_r3;

        // Force on body 1 should point toward body 2 (+x)
        assert!(f1.x > 0.0_f64, "Force on body 1 should point +x, got {:?}", f1);
        // Force on body 2 should point toward body 1 (-x)
        assert!(f2.x < 0.0_f64, "Force on body 2 should point -x, got {:?}", f2);

        // Magnitudes should match the analytical value
        let rel_err = (f1.x - expected_fx).abs() / expected_fx;
        assert!(
            rel_err < 1e-10_f64,
            "Barnes-Hut 2-body force should match analytical: got {}, expected {}, rel_err {}",
            f1.x, expected_fx, rel_err
        );

        // Newton's 3rd law
        let diff = (f1 + f2).length();
        assert!(
            diff < 1e-20_f64,
            "Newton's 3rd law violated: F1 + F2 = {:?}",
            f1 + f2
        );
    }

    /// Four-body test: verify that Barnes-Hut with θ=0 produces
    /// the same result as brute-force (no approximation allowed).
    #[test]
    fn test_barnes_hut_theta_zero_equals_brute_force() {
        let mut world_bh = World::new();
        let mut world_bf = World::new();

        let config_bh = UniverseConfig {
            gravitational_constant: 6.67430e-11_f64,
            softening_epsilon: 1e-4_f64,
            barnes_hut_theta: 0.0_f64, // θ=0 → exact
            sector_size: 1e12_f64,
            ..UniverseConfig::default()
        };
        let config_bf = config_bh.clone();

        world_bh.insert_resource(config_bh);
        world_bf.insert_resource(config_bf);

        // Four bodies at different positions
        let bodies = [
            (1e10_f64, DVec3::new(0.0, 0.0, 0.0)),
            (2e10_f64, DVec3::new(100.0, 0.0, 0.0)),
            (3e10_f64, DVec3::new(0.0, 200.0, 0.0)),
            (5e10_f64, DVec3::new(150.0, 150.0, 0.0)),
        ];

        let mut entities_bh = Vec::new();
        let mut entities_bf = Vec::new();

        for (mass, pos) in &bodies {
            entities_bh.push(world_bh.spawn((
                Force(DVec3::ZERO),
                Mass(*mass),
                Sector::ORIGIN,
                LocalPosition(*pos),
            )).id());
            entities_bf.push(world_bf.spawn((
                Force(DVec3::ZERO),
                Mass(*mass),
                Sector::ORIGIN,
                LocalPosition(*pos),
            )).id());
        }

        // Run both
        let mut schedule_bh = Schedule::default();
        schedule_bh.add_systems(barnes_hut_gravity_system);
        schedule_bh.run(&mut world_bh);

        let mut schedule_bf = Schedule::default();
        schedule_bf.add_systems(super::super::gravity::brute_force_gravity_system);
        schedule_bf.run(&mut world_bf);

        // Compare forces on every body
        for i in 0..bodies.len() {
            let f_bh = world_bh.get::<Force>(entities_bh[i]).unwrap().0;
            let f_bf = world_bf.get::<Force>(entities_bf[i]).unwrap().0;

            let diff = (f_bh - f_bf).length();
            let scale = f_bf.length().max(1e-30_f64);
            let rel_err = diff / scale;

            assert!(
                rel_err < 1e-8_f64,
                "Body {} force mismatch: BH={:?}, BF={:?}, rel_err={:.2e}",
                i, f_bh, f_bf, rel_err
            );
        }
    }

    /// Verify that θ=0.5 produces a reasonable approximation
    /// for a cluster of bodies: error should be bounded.
    #[test]
    fn test_barnes_hut_approximation_bounded_error() {
        let mut world_bh = World::new();
        let mut world_bf = World::new();

        let config_bh = UniverseConfig {
            gravitational_constant: 6.67430e-11_f64,
            softening_epsilon: 1e-4_f64,
            barnes_hut_theta: 0.5_f64,
            sector_size: 1e12_f64,
            ..UniverseConfig::default()
        };
        let config_bf = config_bh.clone();
        let config_bf2 = UniverseConfig { barnes_hut_theta: 0.0_f64, ..config_bf };

        world_bh.insert_resource(config_bh);
        world_bf.insert_resource(config_bf2);

        // 16 bodies in a loose cluster + 1 distant body
        let mut entities_bh = Vec::new();
        let mut entities_bf = Vec::new();

        // Cluster near origin
        for i in 0..4 {
            for j in 0..4 {
                let pos = DVec3::new(i as f64 * 10.0, j as f64 * 10.0, 0.0);
                let mass = 1e8_f64;
                entities_bh.push(world_bh.spawn((
                    Force(DVec3::ZERO), Mass(mass), Sector::ORIGIN, LocalPosition(pos),
                )).id());
                entities_bf.push(world_bf.spawn((
                    Force(DVec3::ZERO), Mass(mass), Sector::ORIGIN, LocalPosition(pos),
                )).id());
            }
        }

        // Distant body
        let far_pos = DVec3::new(1000.0, 0.0, 0.0);
        entities_bh.push(world_bh.spawn((
            Force(DVec3::ZERO), Mass(5e10_f64), Sector::ORIGIN, LocalPosition(far_pos),
        )).id());
        entities_bf.push(world_bf.spawn((
            Force(DVec3::ZERO), Mass(5e10_f64), Sector::ORIGIN, LocalPosition(far_pos),
        )).id());

        // Run both
        let mut schedule_bh = Schedule::default();
        schedule_bh.add_systems(barnes_hut_gravity_system);
        schedule_bh.run(&mut world_bh);

        let mut schedule_bf = Schedule::default();
        schedule_bf.add_systems(barnes_hut_gravity_system); // θ=0 acts as brute force
        schedule_bf.run(&mut world_bf);

        // The distant body should see ~same force from both methods
        let idx_far = entities_bh.len() - 1;
        let f_bh = world_bh.get::<Force>(entities_bh[idx_far]).unwrap().0;
        let f_bf = world_bf.get::<Force>(entities_bf[idx_far]).unwrap().0;

        let rel_err = (f_bh - f_bf).length() / f_bf.length().max(1e-30_f64);
        assert!(
            rel_err < 0.05_f64, // θ=0.5 should give <5% error for well-separated clusters
            "Distant body force approximation error too large: {:.2}%",
            rel_err * 100.0
        );
    }
}
