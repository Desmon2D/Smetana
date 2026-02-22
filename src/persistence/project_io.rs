use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::model::Project;

const PROJECTS_DIR: &str = "saves/projects";

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
    fs::create_dir_all("saves/prices")
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
    serde_json::from_str(&json).map_err(|e| format!("Ошибка десериализации: {e}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Point2D, Wall};
    use std::fs;

    #[test]
    fn round_trip_project_with_wall() {
        let mut project = Project::new("_test_round_trip".to_string());
        project.walls.push(Wall::new(
            Point2D::new(0.0, 0.0),
            Point2D::new(4000.0, 0.0),
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
}
