use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unit of measurement for a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitType {
    /// Per piece (штука)
    Piece,
    /// Per square meter (м²)
    SquareMeter,
    /// Per linear meter (п.м.)
    LinearMeter,
}

impl UnitType {
    pub const ALL: [UnitType; 3] = [UnitType::Piece, UnitType::SquareMeter, UnitType::LinearMeter];

    pub fn label(self) -> &'static str {
        match self {
            UnitType::Piece => "шт.",
            UnitType::SquareMeter => "м²",
            UnitType::LinearMeter => "п.м.",
        }
    }
}

/// Which object type a service applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetObjectType {
    Wall,
    Window,
    Door,
    Room,
}

impl TargetObjectType {
    pub const ALL: [TargetObjectType; 4] = [
        TargetObjectType::Wall,
        TargetObjectType::Window,
        TargetObjectType::Door,
        TargetObjectType::Room,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TargetObjectType::Wall => "Стена",
            TargetObjectType::Window => "Окно",
            TargetObjectType::Door => "Дверь",
            TargetObjectType::Room => "Помещение",
        }
    }
}

/// A service definition within a price list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTemplate {
    pub id: Uuid,
    /// Service name (e.g. "Штукатурка стен")
    pub name: String,
    /// Unit of measurement
    pub unit_type: UnitType,
    /// Price per unit (rubles)
    pub price_per_unit: f64,
    /// Which object type this service applies to
    pub target_type: TargetObjectType,
}

impl ServiceTemplate {
    pub fn new(name: String, unit_type: UnitType, price_per_unit: f64, target_type: TargetObjectType) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            unit_type,
            price_per_unit,
            target_type,
        }
    }
}

/// A named collection of service templates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceList {
    pub id: Uuid,
    pub name: String,
    pub services: Vec<ServiceTemplate>,
}

impl PriceList {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            services: Vec::new(),
        }
    }
}
