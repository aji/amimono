use std::{collections::HashMap, path::PathBuf};

use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};

use crate::{
    component::Location,
    error::{Error, Result},
    runtime::{self, RuntimeProvider},
};

#[derive(Serialize, Deserialize)]
struct StaticConfig {
    job: HashMap<String, StaticJobConfig>,
}

#[derive(Serialize, Deserialize)]
struct StaticJobConfig {
    locations: Vec<String>,
}

pub struct StaticRuntime {
    root: PathBuf,
    myself: Location,
}

impl StaticRuntime {
    pub fn open(root: PathBuf, myself: Location) -> StaticRuntime {
        StaticRuntime { root, myself }
    }

    async fn config(&self) -> Result<StaticConfig> {
        let config_path = self.root.join("amimono.toml");
        let config = tokio::fs::read(&config_path)
            .await
            .map_err(|_| "could not read config")?;
        toml::from_slice(&config[..]).map_err(|_| Error::from("could not parse config"))
    }

    async fn discover_inner(&self, component: &str) -> Result<Vec<Location>> {
        let job = runtime::config()
            .component_job(component)
            .ok_or("component has no job")?;
        let res = self
            .config()
            .await?
            .job
            .get(job)
            .ok_or("static config missing job")?
            .locations
            .iter()
            .cloned()
            .map(|x| Location::stable(x))
            .collect();
        Ok(res)
    }

    async fn myself_inner(&self, _component: &str) -> Result<Location> {
        Ok(self.myself.clone())
    }

    async fn storage_inner(&self, component: &str) -> Result<PathBuf> {
        let myself: &str = self.myself.addr();
        let dir = self.root.join("storage").join(myself).join(component);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|_| "could not create storage dir")?;
        Ok(dir)
    }
}

impl RuntimeProvider for StaticRuntime {
    fn discover_running<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>> {
        Box::pin(self.discover_inner(component))
    }

    fn discover_stable<'f, 'p: 'f, 'l: 'f>(
        &'p self,
        component: &'l str,
    ) -> BoxFuture<'f, Result<Vec<Location>>> {
        Box::pin(self.discover_inner(component))
    }

    fn myself<'f, 'p: 'f, 'l: 'f>(&'p self, component: &'l str) -> BoxFuture<'f, Result<Location>> {
        Box::pin(self.myself_inner(component))
    }

    fn storage<'f, 'p: 'f, 'l: 'f>(&'p self, component: &'l str) -> BoxFuture<'f, Result<PathBuf>> {
        Box::pin(self.storage_inner(component))
    }
}
