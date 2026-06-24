use serde::{Serialize, Deserialize};
use bevy_ecs::prelude::*;
use crate::components::*;
use crate::core::coordinates::{LocalPosition, Sector};

#[derive(Serialize, Deserialize)]
pub struct SavedEntity {
    pub name: Option<identifiers::EntityName>,
    pub body_type: Option<identifiers::BodyType>,
    pub mass: Option<Mass>,
    pub radius: Option<spatial::BoundingRadius>,
    pub local_pos: Option<LocalPosition>,
    pub sector: Option<Sector>,
    pub velocity: Option<Velocity>,
    pub temperature: Option<Temperature>,
}

#[derive(Serialize, Deserialize)]
pub struct SavedSimulation {
    pub entities: Vec<SavedEntity>,
}
