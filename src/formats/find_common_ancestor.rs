use std::path::PathBuf;
pub fn find_common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    let mut common_ancestor = paths[0].clone();

    for path in paths.iter().skip(1) {
        while !path.starts_with(&common_ancestor) {
            if let Some(parent) = common_ancestor.parent() {
                common_ancestor = parent.to_path_buf();
            } else {
                return Some(PathBuf::from("/")); // Reached root, no common ancestor other than root
            }
        }
    }
    Some(common_ancestor)
}