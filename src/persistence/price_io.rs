use std::fs;
use std::path::{Path, PathBuf};

use crate::model::PriceList;

use super::project_io::ensure_saves_dirs;

const PRICES_DIR: &str = "saves/prices";

/// Build the file path for a price list by name.
pub fn price_path(name: &str) -> PathBuf {
    Path::new(PRICES_DIR).join(format!("{name}.json"))
}

/// Save a price list to `saves/prices/{name}.json`.
pub fn save_price_list(price_list: &PriceList) -> Result<PathBuf, String> {
    ensure_saves_dirs()?;
    let path = price_path(&price_list.name);
    let json = serde_json::to_string_pretty(price_list)
        .map_err(|e| format!("Ошибка сериализации прайс-листа: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("Ошибка записи файла: {e}"))?;
    Ok(path)
}

/// Save a price list to an arbitrary path.
pub fn save_price_list_to(price_list: &PriceList, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(price_list)
        .map_err(|e| format!("Ошибка сериализации прайс-листа: {e}"))?;
    fs::write(path, &json).map_err(|e| format!("Ошибка записи файла: {e}"))?;
    Ok(())
}

/// Load a price list from a JSON file.
pub fn load_price_list(path: &Path) -> Result<PriceList, String> {
    let json =
        fs::read_to_string(path).map_err(|e| format!("Ошибка чтения файла: {e}"))?;
    serde_json::from_str(&json).map_err(|e| format!("Ошибка десериализации прайс-листа: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ServiceTemplate, TargetObjectType, UnitType};
    use std::fs;

    #[test]
    fn round_trip_price_list() {
        let mut pl = PriceList::new("_test_prices".to_string());
        pl.services.push(ServiceTemplate::new(
            "Штукатурка стен".to_string(),
            UnitType::SquareMeter,
            450.0,
            TargetObjectType::Wall,
        ));
        pl.services.push(ServiceTemplate::new(
            "Установка двери".to_string(),
            UnitType::Piece,
            3500.0,
            TargetObjectType::Door,
        ));

        let path = save_price_list(&pl).expect("save failed");
        assert!(path.exists());

        let loaded = load_price_list(&path).expect("load failed");
        assert_eq!(loaded.id, pl.id);
        assert_eq!(loaded.name, pl.name);
        assert_eq!(loaded.services.len(), 2);
        assert_eq!(loaded.services[0].name, "Штукатурка стен");
        assert_eq!(loaded.services[1].price_per_unit, 3500.0);

        let _ = fs::remove_file(&path);
    }
}
