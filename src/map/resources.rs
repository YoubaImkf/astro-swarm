use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Energy,
    Minerals,
    SciencePoints,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub amount: u32,
}

impl Resource {
    pub fn new(resource_type: ResourceType, amount: u32) -> Self {
        Self { resource_type, amount }
    }
}

pub struct ResourceManager {
    resources: HashMap<(usize, usize), Resource>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn get_all_resources(&self) -> &HashMap<(usize, usize), Resource> {
        &self.resources
    }

    pub fn get_resource(&self, x: usize, y: usize) -> Option<&Resource> {
        self.resources.get(&(x, y))
    }

    pub fn remove_resource(&mut self, x: usize, y: usize) -> Option<Resource> {
        self.resources.remove(&(x, y))
    }

    pub fn has_resource(&self, x: usize, y: usize) -> bool {
        self.resources.contains_key(&(x, y))
    }

    pub fn add_resource(&mut self, x: usize, y: usize, resource_type: ResourceType, amount: u32) {
        self.resources.insert((x, y), Resource::new(resource_type, amount));
    }

    pub fn collect_resource(&mut self, x: usize, y: usize) -> Option<Resource> {
        self.resources.remove(&(x, y))
    }
}