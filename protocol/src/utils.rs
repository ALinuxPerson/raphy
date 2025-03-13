use std::env;
use std::path::PathBuf;

pub fn auto_detect_java_from_java_home_env() -> Option<PathBuf> {
    env::var("JAVA_HOME")
        .ok()
        .map(|java_home| PathBuf::from(java_home).join("bin").join("java"))
}

pub fn auto_detect_java_from_system_path() -> Option<PathBuf> {
    const JAVA_EXE: &str = if cfg!(windows) { "java.exe" } else { "java" };

    env::var("PATH").ok().and_then(|path| {
        env::split_paths(&path)
            .filter_map(|dir| {
                let java_path = dir.join(JAVA_EXE);
                if java_path.is_file() {
                    Some(java_path)
                } else {
                    None
                }
            })
            .next()
    })
}
