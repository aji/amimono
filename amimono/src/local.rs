use std::path::PathBuf;

use futures::future::BoxFuture;

use crate::{component::Location, error::Result, runtime};

pub struct LocalRuntime {
    root: PathBuf,
}

impl LocalRuntime {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        LocalRuntime {
            root: root.into().join(".amimono"),
        }
    }
}

impl runtime::RuntimeProvider for LocalRuntime {
    fn discover_running<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _label: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>> {
        Box::pin(async { Ok(vec![Location::Stable("localhost".to_owned())]) })
    }

    fn discover_stable<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        _label: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>> {
        Box::pin(async { Ok(vec![Location::Stable("localhost".to_owned())]) })
    }

    fn myself<'f, 'p: 'f, 'l: 'f>(&'p self, _label: &'l str) -> BoxFuture<'f, Result<Location>> {
        Box::pin(async { Ok(Location::Stable("localhost".to_owned())) })
    }

    fn storage<'f, 'p: 'f, 'l: 'f>(&'p self, component: &'l str) -> BoxFuture<'f, Result<PathBuf>> {
        Box::pin(async move {
            let dir = self.root.join("storage").join(component);
            if !dir.exists() {
                if let Err(_) = std::fs::create_dir_all(&dir) {
                    log::error!(
                        "failed to create storage dir for component {}: {:?}",
                        component,
                        dir
                    );
                    Err("failed to create storage dir for component")?;
                }
            }
            Ok(dir)
        })
    }
}
