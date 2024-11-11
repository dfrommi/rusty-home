use std::path::PathBuf;

pub fn find_file_upwards(file_name: &str) -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;

    // Iterate over ancestors, starting from the current directory
    for dir in current_dir.ancestors() {
        let file_path = dir.join(file_name);
        if file_path.exists() {
            return Some(file_path);
        }
    }

    None
}
