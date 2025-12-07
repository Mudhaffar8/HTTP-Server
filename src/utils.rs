use std::path::{Path, Component};

/// Checks whether a file system path is safe to serve
/// 
/// Path is considered safe iff it:
/// - Does not contain `".."` components (prevents parent directory traversal)
/// - Does not start with an absolute root
/// - Does not contain prefix components (such as Windows `C:\` drives)
pub fn is_path_safe(path : &str) -> bool {

    let path = Path::new(path);

   for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,

            Component::CurDir | Component::Normal(_) => continue
        }
   }

    true
}
