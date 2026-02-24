use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::model::{PriceList, Project};

const PROJECTS_DIR: &str = "saves/projects";
const PRICES_DIR: &str = "saves/prices";

// --- Project I/O ---

/// Summary info for a project file (for the project list screen).
pub struct ProjectEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

/// Ensure both save directories exist.
pub fn ensure_saves_dirs() -> Result<(), String> {
    fs::create_dir_all(PROJECTS_DIR)
        .map_err(|e| format!("Не удалось создать каталог проектов: {e}"))?;
    fs::create_dir_all(PRICES_DIR)
        .map_err(|e| format!("Не удалось создать каталог прайс-листов: {e}"))?;
    Ok(())
}

/// Build the file path for a project by name.
pub fn project_path(name: &str) -> PathBuf {
    Path::new(PROJECTS_DIR).join(format!("{name}.json"))
}

/// Save a project to `saves/projects/{name}.json`.
pub fn save_project(project: &Project) -> Result<PathBuf, String> {
    ensure_saves_dirs()?;
    let path = project_path(&project.name);
    let json = serde_json::to_string_pretty(project)
        .map_err(|e| format!("Ошибка сериализации: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("Ошибка записи файла: {e}"))?;
    Ok(path)
}

/// Load a project from a JSON file.
pub fn load_project(path: &Path) -> Result<Project, String> {
    let json = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения файла: {e}"))?;
    let mut project: Project =
        serde_json::from_str(&json).map_err(|e| format!("Ошибка десериализации: {e}"))?;
    // Post-deserialization fixup: ensure all wall sides have sections
    // (old saves may have empty sections vec).
    for wall in &mut project.walls {
        wall.left_side.ensure_sections();
        wall.right_side.ensure_sections();
    }
    Ok(project)
}

/// List all project JSON files in the saves directory.
pub fn list_projects() -> Result<Vec<PathBuf>, String> {
    ensure_saves_dirs()?;
    let entries =
        fs::read_dir(PROJECTS_DIR).map_err(|e| format!("Ошибка чтения каталога: {e}"))?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    Ok(paths)
}

/// List projects with metadata (name derived from filename, last modified date).
pub fn list_project_entries() -> Result<Vec<ProjectEntry>, String> {
    let paths = list_projects()?;
    let mut entries = Vec::new();
    for path in paths {
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let modified = fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        entries.push(ProjectEntry {
            name,
            path,
            modified,
        });
    }
    // Sort by last modified, newest first
    entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(entries)
}

/// Delete a project JSON file.
pub fn delete_project(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| format!("Ошибка удаления: {e}"))
}

// --- Price List I/O ---

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

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec2;
    use crate::model::{Wall, ServiceTemplate, TargetObjectType, UnitType};
    use std::fs;

    #[test]
    fn round_trip_project_with_wall() {
        let mut project = Project::new("_test_round_trip".to_string());
        project.walls.push(Wall::new(
            DVec2::new(0.0, 0.0),
            DVec2::new(4000.0, 0.0),
            200.0,
            2700.0,
        ));

        // Save
        let path = save_project(&project).expect("save failed");
        assert!(path.exists());

        // Load
        let loaded = load_project(&path).expect("load failed");
        assert_eq!(loaded.id, project.id);
        assert_eq!(loaded.name, project.name);
        assert_eq!(loaded.walls.len(), 1);
        assert_eq!(loaded.walls[0].id, project.walls[0].id);
        assert_eq!(loaded.walls[0].start.x, 0.0);
        assert_eq!(loaded.walls[0].end.x, 4000.0);
        assert_eq!(loaded.walls[0].thickness, 200.0);
        assert_eq!(loaded.walls[0].left_side.height_start, 2700.0);

        // Cleanup
        let _ = fs::remove_file(&path);
    }

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
