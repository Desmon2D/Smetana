use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{Label, Opening, Room, Wall};

/// Default dimensions used when creating new walls, doors, and windows in this project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectDefaults {
    /// Wall thickness in mm
    pub wall_thickness: f64,
    /// Wall height in mm
    pub wall_height: f64,
    /// Door height in mm
    pub door_height: f64,
    /// Door width in mm
    pub door_width: f64,
    /// Window height in mm
    pub window_height: f64,
    /// Window width in mm
    pub window_width: f64,
    /// Window sill height in mm
    pub window_sill_height: f64,
    /// Window reveal width in mm
    pub window_reveal_width: f64,
}

impl Default for ProjectDefaults {
    fn default() -> Self {
        Self {
            wall_thickness: 200.0,
            wall_height: 2700.0,
            door_height: 2100.0,
            door_width: 900.0,
            window_height: 1400.0,
            window_width: 1200.0,
            window_sill_height: 900.0,
            window_reveal_width: 250.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignedService {
    pub service_template_id: Uuid,
    /// Overridden price (if None — taken from template)
    pub custom_price: Option<f64>,
}

/// Services assigned to one side of a wall.
/// `sections` has one entry per section (no T-junctions = exactly 1 section).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideServices {
    pub sections: Vec<Vec<AssignedService>>,
}

impl SideServices {
    /// Ensure at least one section exists, return mutable ref to section at index.
    pub fn ensure_section(&mut self, section_index: usize) -> &mut Vec<AssignedService> {
        while self.sections.len() <= section_index {
            self.sections.push(Vec::new());
        }
        &mut self.sections[section_index]
    }

    /// Get all services across all sections (flat iterator).
    pub fn all_services(&self) -> impl Iterator<Item = &AssignedService> {
        self.sections.iter().flat_map(|s| s.iter())
    }

    /// Check if any section has services.
    pub fn is_empty(&self) -> bool {
        self.sections.iter().all(|s| s.is_empty())
    }
}

/// Per-side services for a wall (left and right).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WallSideServices {
    pub left: SideServices,
    pub right: SideServices,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub rooms: Vec<Room>,
    #[serde(default)]
    pub labels: Vec<Label>,
    /// ID of the price list in use
    pub price_list_id: Option<Uuid>,
    /// Assigned services by wall ID (per-side)
    #[serde(default)]
    pub wall_services: HashMap<Uuid, WallSideServices>,
    /// Assigned services by opening ID
    pub opening_services: HashMap<Uuid, Vec<AssignedService>>,
    /// Assigned services by room ID
    pub room_services: HashMap<Uuid, Vec<AssignedService>>,
    /// Default dimensions for new walls/openings
    #[serde(default)]
    pub defaults: ProjectDefaults,
}

impl Project {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            walls: Vec::new(),
            openings: Vec::new(),
            rooms: Vec::new(),
            labels: Vec::new(),
            price_list_id: None,
            wall_services: HashMap::new(),
            opening_services: HashMap::new(),
            room_services: HashMap::new(),
            defaults: ProjectDefaults::default(),
        }
    }
}
