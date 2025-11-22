use std::{hash::Hash, hash::Hasher, path::PathBuf};

/// A helper for `build.rs` scripts to compute an app revision.
pub struct AppDigest {
    paths: Vec<PathBuf>,
}

impl AppDigest {
    pub fn new() -> Self {
        AppDigest { paths: Vec::new() }
    }

    pub fn add_path<S: Into<PathBuf>>(&mut self, path: S) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    pub fn add_paths<I: IntoIterator<Item = S>, S: Into<PathBuf>>(
        &mut self,
        paths: I,
    ) -> &mut Self {
        self.paths.extend(paths.into_iter().map(|p| p.into()));
        self
    }

    pub fn add_glob<S: AsRef<str>>(&mut self, pattern: S) -> &mut Self {
        let paths = glob::glob(pattern.as_ref()).expect("failed to read glob pattern");
        self.add_paths(paths.map(|p| p.expect("failed to read glob entry")));
        self
    }

    pub fn compute(&mut self) -> String {
        self.paths.sort();

        let mut hasher = fnv::FnvHasher::default();
        for path in self.paths.iter() {
            // TODO: include paths in the hash
            std::fs::read(&path)
                .unwrap_or_else(|e| panic!("could not read {:?}: {}", path, e))
                .hash(&mut hasher);
        }

        format!("{:08x}", hasher.finish() & 0xffffffff)
    }
}
