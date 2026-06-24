use bevy_ecs::prelude::Resource;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Resource, Default)]
pub struct Diagnostics {
    pub timings: HashMap<String, Duration>,
}

impl Diagnostics {
    pub fn record(&mut self, name: &str, duration: Duration) {
        self.timings.insert(name.to_string(), duration);
    }
}
