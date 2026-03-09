use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::model::Project;

const PROJECTS_DIR: &str = "saves/projects";

// --- Project I/O ---

/// Summary info for a project file (for the project list screen).
pub struct ProjectEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

/// Ensure save directories exist.
pub fn ensure_saves_dirs() -> Result<(), String> {
    fs::create_dir_all(PROJECTS_DIR)
        .map_err(|e| format!("Не удалось создать каталог проектов: {e}"))?;
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
    let json =
        serde_json::to_string_pretty(project).map_err(|e| format!("Ошибка сериализации: {e}"))?;
    fs::write(&path, &json).map_err(|e| format!("Ошибка записи файла: {e}"))?;
    Ok(path)
}

/// Load a project from a JSON file.
pub fn load_project(path: &Path) -> Result<Project, String> {
    let json = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения файла: {e}"))?;
    let project: Project =
        serde_json::from_str(&json).map_err(|e| format!("Ошибка десериализации: {e}"))?;
    Ok(project)
}

/// List projects with metadata (name derived from filename, last modified date).
pub fn list_project_entries() -> Result<Vec<ProjectEntry>, String> {
    ensure_saves_dirs()?;
    let dir_entries =
        fs::read_dir(PROJECTS_DIR).map_err(|e| format!("Ошибка чтения каталога: {e}"))?;
    let mut entries: Vec<ProjectEntry> = dir_entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .map(|path| {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let modified = fs::metadata(&path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            ProjectEntry {
                name,
                path,
                modified,
            }
        })
        .collect();
    // Sort by last modified, newest first
    entries.sort_by_key(|e| std::cmp::Reverse(e.modified));
    Ok(entries)
}

/// Delete a project JSON file.
pub fn delete_project(path: &Path) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| format!("Ошибка удаления: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use glam::DVec2;
    use uuid::Uuid;

    #[test]
    fn round_trip_project() {
        // Create a project with all object types
        let mut project = Project::new("test_round_trip".to_string());

        // Add 4 points
        let ids: Vec<Uuid> = (0..4).map(|_| Uuid::new_v4()).collect();
        let positions = [
            DVec2::new(0.0, 0.0),
            DVec2::new(3000.0, 0.0),
            DVec2::new(3000.0, 4000.0),
            DVec2::new(0.0, 4000.0),
        ];
        for (i, &id) in ids.iter().enumerate() {
            project.points.push(Point {
                id,
                position: positions[i],
                height: 2700.0,
            });
        }

        // Ensure edges for the contour
        project.ensure_contour_edges(&ids);
        assert_eq!(project.edges.len(), 4);

        // Add a room
        let room = Room::new("Кухня".to_string(), ids.clone(), Room::default_color());
        let room_id = room.id;
        project.rooms.push(room);

        // Add a wall
        let wall = Wall::new(ids.clone(), [180, 180, 180, 255]);
        let wall_id = wall.id;
        project.walls.push(wall);

        // Add an opening (door)
        let opening = Opening::new(
            vec![ids[0], ids[1]],
            OpeningKind::Door {
                height: 2100.0,
                width: 900.0,
                reveal_width: 0.0,
                swing_edge: 0,
                swing_outward: true,
                swing_mirrored: false,
                show_swing: true,
            },
            [210, 170, 120, 200],
        );
        let opening_id = opening.id;
        project.openings.push(opening);

        // Add a label
        let label = Label::new("Test Label".to_string(), DVec2::new(1500.0, 2000.0));
        let label_id = label.id;
        project.labels.push(label);

        // Set an edge distance override
        project.edges[0].distance_override = Some(3200.0);

        // Save
        let path = save_project(&project).expect("save failed");

        // Load
        let loaded = load_project(&path).expect("load failed");

        // Verify
        assert_eq!(loaded.name, "test_round_trip");
        assert_eq!(loaded.points.len(), 4);
        assert_eq!(loaded.edges.len(), 4);
        assert_eq!(loaded.rooms.len(), 1);
        assert_eq!(loaded.walls.len(), 1);
        assert_eq!(loaded.openings.len(), 1);
        assert_eq!(loaded.labels.len(), 1);

        // Verify specific fields
        let loaded_room = loaded.room(room_id).expect("room not found");
        assert_eq!(loaded_room.name, "Кухня");
        assert_eq!(loaded_room.points.len(), 4);

        let loaded_wall = loaded.wall(wall_id).expect("wall not found");
        assert_eq!(loaded_wall.points.len(), 4);
        assert_eq!(loaded_wall.color, [180, 180, 180, 255]);

        let loaded_opening = loaded.opening(opening_id).expect("opening not found");
        assert!(
            matches!(loaded_opening.kind, OpeningKind::Door { height, width, .. } if height == 2100.0 && width == 900.0)
        );

        let loaded_label = loaded
            .labels
            .iter()
            .find(|l| l.id == label_id)
            .expect("label not found");
        assert_eq!(loaded_label.text, "Test Label");

        // Verify edge override
        assert_eq!(loaded.edges[0].distance_override, Some(3200.0));

        // Verify defaults round-trip
        assert_eq!(loaded.defaults.point_height, 2700.0);

        // Cleanup
        let _ = fs::remove_file(&path);
    }
}
