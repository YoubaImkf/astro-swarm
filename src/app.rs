use crate::map::noise;

pub struct App {
    pub map: noise::Map,
}

impl App {
    /// Creates the map using the given seeds,
    /// spawns resources and returns the App.
    pub fn new(width: usize, height: usize, map_seed: u32, resource_seed: u64) -> Self {
        let mut map = noise::Map::new(width, height, map_seed);
        map.spawn_resources(20, resource_seed);
        Self { map }
    }

    pub fn quit(&self) -> bool {
        false
    }
}