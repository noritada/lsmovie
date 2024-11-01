use std::{
    env, fs, io,
    path::{Component, Path},
    sync::LazyLock,
};

use regex::Regex;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct MovieEntry {
    id: String,
    user: String,
    title: String,
}

impl MovieEntry {
    fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
        let path = path.as_ref();
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let (id, title) = extract_id(stem)?;
        let (id, title) = (id.to_owned(), title.to_owned());
        let user = path
            .components()
            .rev()
            .skip(1)
            .find_map(|component| extract_user_name(&component))?;
        Some(MovieEntry { id, user, title })
    }
}

fn extract_id(stem: &str) -> Option<(&str, &str)> {
    static RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?<title>.+)\s+\[(?<id>[^\]]+)\]$").unwrap());
    let caps = RE.captures(stem)?;
    let title = caps.name("title")?.as_str();
    let id = caps.name("id")?.as_str();
    Some((id, title))
}

fn extract_user_name(component: &Component) -> Option<String> {
    let s = component.as_os_str().to_str()?;
    if s.starts_with("@") {
        Some(s.to_owned())
    } else {
        None
    }
}

fn visit_dir<P: AsRef<Path>>(dir: P, cb: &dyn Fn(&fs::DirEntry)) -> io::Result<()> {
    let dir = dir.as_ref();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit_dir(path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }
    Ok(())
}

const EXTENSIONS: [&'static str; 3] = ["mkv", "mp4", "webm"];

fn process(entry: &fs::DirEntry) {
    let path = entry.path();
    if let Some(ext) = path.extension() {
        if !EXTENSIONS.contains(&ext.to_str().unwrap_or_default()) {
            eprintln!("ignored: {:?}", &path);
            return;
        }
    }
    if let Some(entry) = MovieEntry::from_path(&path) {
        let j = serde_json::to_string(&entry).expect("JSON serialization failed");
        println!("{}", j)
    } else {
        eprintln!("movie info extraction failed: {:?}", &path)
    }
}

fn main() {
    let args = env::args().skip(1);
    for arg in args {
        let _ = visit_dir(arg, &process);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movie_info_extraction() {
        let path = Path::new("./@path/to/@foobar/baz/@FooBar (2024年11月1日) [aBcDeFgHiJkL].webm");
        let actual = MovieEntry::from_path(&path);
        let expected = Some(MovieEntry {
            id: "aBcDeFgHiJkL".to_owned(),
            user: "@foobar".to_owned(),
            title: "@FooBar (2024年11月1日)".to_owned(),
        });
        assert_eq!(actual, expected);
    }
}
