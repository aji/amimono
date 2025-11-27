use std::path::PathBuf;

use futures::future::BoxFuture;

use crate::{
    config::Binding,
    runtime::{self, Location, RuntimeResult},
};

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
    fn discover<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        label: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<Location>> {
        let binding = runtime::binding_by_label(label);
        let res = match binding {
            Binding::None => Err("component has no binding"),
            Binding::Http(port) => {
                let url = format!("http://localhost:{}", port);
                Ok(Location::Http(url))
            }
        };
        Box::pin(async { res })
    }

    fn discover_all<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        label: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<Vec<Location>>> {
        Box::pin(async move {
            let res = self.discover(label).await?;
            Ok(vec![res])
        })
    }

    fn storage<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, RuntimeResult<PathBuf>> {
        Box::pin(async move {
            let dir = self.root.join("storage").join(component);
            if !dir.exists() {
                if let Err(_) = std::fs::create_dir_all(&dir) {
                    log::error!(
                        "failed to create storage dir for component {}: {:?}",
                        component,
                        dir
                    );
                    return Err("failed to create storage dir for component");
                }
            }
            Ok(dir)
        })
    }
}
