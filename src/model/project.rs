use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{Opening, Room, Wall};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedService {
    pub service_template_id: Uuid,
    /// Overridden price (if None — taken from template)
    pub custom_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub rooms: Vec<Room>,
    /// ID of the price list in use
    pub price_list_id: Option<Uuid>,
    /// Assigned services by wall ID
    pub wall_services: HashMap<Uuid, Vec<AssignedService>>,
    /// Assigned services by opening ID
    pub opening_services: HashMap<Uuid, Vec<AssignedService>>,
    /// Assigned services by room ID
    pub room_services: HashMap<Uuid, Vec<AssignedService>>,
}

impl Project {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            walls: Vec::new(),
            openings: Vec::new(),
            rooms: Vec::new(),
            price_list_id: None,
            wall_services: HashMap::new(),
            opening_services: HashMap::new(),
            room_services: HashMap::new(),
        }
    }
}
