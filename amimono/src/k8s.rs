use std::{
    collections::{HashMap, HashSet},
    fmt,
    ops::Deref,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use futures::{StreamExt, future::BoxFuture};
use kube::{
    Api, ResourceExt,
    api::{ObjectList, WatchEvent},
};
use rand::seq::IndexedRandom;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::{
    config::Binding,
    runtime::{self, Location, RuntimeResult},
};

pub struct K8sRuntime {
    discovery_cache: Arc<K8sWatcher<DiscoveryCache>>,
}

impl K8sRuntime {
    pub async fn new(namespace: String, config: kube::config::Config) -> Self {
        let client = kube::Client::try_from(config).expect("failed to create Kubernetes client");

        let discovery_cache = K8sWatcher::new(
            Api::namespaced(client.clone(), &namespace),
            DiscoveryCache::new(),
        )
        .await;
        discovery_cache.start();

        K8sRuntime { discovery_cache }
    }

    async fn discover_inner(&self, component: &'static str) -> RuntimeResult<Location> {
        let binding = runtime::binding_by_label(component);
        let job = runtime::config()
            .component_job(component)
            .ok_or("component has no job")?;

        let cache = self.discovery_cache.read().await;

        let pod_ip = cache
            .pods_by_job
            .get(job)
            .iter()
            .flat_map(|names| names.iter())
            .collect::<Vec<_>>()
            .choose(&mut rand::rng())
            .and_then(|name| cache.pods.get(name.as_str()))
            .map(|pod| pod.ip.as_str());

        match binding {
            Binding::None => Err("component has no binding"),
            Binding::Http(port) => {
                let ip = match pod_ip {
                    Some(ip) => ip,
                    None => return Err("no pods found for component"),
                };
                let url = format!("http://{}:{}", ip, port);
                Ok(Location::Http(url))
            }
        }
    }

    async fn discover_all_inner(&self, component: &'static str) -> RuntimeResult<Vec<Location>> {
        let binding = runtime::binding_by_label(component);
        let job = runtime::config()
            .component_job(component)
            .ok_or("component has no job")?;

        let cache = self.discovery_cache.read().await;

        let pod_ips = cache
            .pods_by_job
            .get(job)
            .iter()
            .flat_map(|names| names.iter())
            .filter_map(|name| cache.pods.get(name.as_str()))
            .map(|pod| pod.ip.as_str())
            .collect::<Vec<_>>();

        match binding {
            Binding::None => Ok(Vec::new()),
            Binding::Http(port) => {
                let urls = pod_ips
                    .into_iter()
                    .map(|ip| Location::Http(format!("http://{}:{}", ip, port)))
                    .collect::<Vec<_>>();
                if urls.is_empty() {
                    return Err("no pods found for component");
                }
                Ok(urls)
            }
        }
    }
}

impl runtime::RuntimeProvider for K8sRuntime {
    fn discover(&'_ self, component: &'static str) -> BoxFuture<'_, RuntimeResult<Location>> {
        Box::pin(self.discover_inner(component))
    }

    fn discover_all(
        &'_ self,
        component: &'static str,
    ) -> BoxFuture<'_, RuntimeResult<Vec<Location>>> {
        Box::pin(self.discover_all_inner(component))
    }

    fn storage(&'_ self, _component: &'static str) -> BoxFuture<'_, RuntimeResult<PathBuf>> {
        Box::pin(async { Err("storage() not implemented for k8s runtime") })
    }
}

trait K8sCache: Send + Sync + 'static {
    type Resource: Clone + DeserializeOwned + fmt::Debug + Send + 'static;

    fn reset(&mut self, list: ObjectList<Self::Resource>);
    fn update(&mut self, event: WatchEvent<Self::Resource>);
}

struct DiscoveryCache {
    pods: HashMap<String, DiscoveryCachePod>,
    pods_by_job: HashMap<String, HashSet<String>>,
}

struct DiscoveryCachePod {
    ip: String,
    job: String,
}

enum DiscoveryCacheError {
    Ignored(&'static str),
    Fatal(&'static str),
}

type DiscoveryCacheResult<T> = Result<T, DiscoveryCacheError>;

impl DiscoveryCache {
    fn new() -> Self {
        DiscoveryCache {
            pods: HashMap::new(),
            pods_by_job: HashMap::new(),
        }
    }

    fn insert(&mut self, pod: &k8s_openapi::api::core::v1::Pod) -> DiscoveryCacheResult<()> {
        use DiscoveryCacheError::*;

        let status = pod
            .status
            .as_ref()
            .ok_or(Fatal("could not get pod status"))?;

        let phase = status
            .phase
            .as_deref()
            .ok_or(Fatal("could not get pod phase"))?;

        if phase != "Running" {
            return Err(Ignored("pod is not running"));
        }

        let pod_name = pod
            .metadata
            .name
            .as_deref()
            .ok_or(Fatal("could not get pod name"))?
            .to_owned();
        let pod_labels = pod
            .metadata
            .labels
            .as_ref()
            .ok_or(Fatal("could not get pod labels"))?;

        let job_label = pod_labels
            .get("amimono-job")
            .ok_or(Ignored("pod does not have amimono-job label"))?
            .clone();
        let job_rev = pod_labels
            .get("amimono-rev")
            .ok_or(Ignored("pod does not have amimono-rev label"))?
            .clone();

        if job_rev != runtime::config().revision() {
            return Err(Ignored("pod revision does not match"));
        }

        let pod_ip = status
            .pod_ip
            .as_deref()
            .ok_or(Ignored("pod has no IP"))?
            .to_owned();

        let pod = DiscoveryCachePod {
            ip: pod_ip,
            job: job_label.clone(),
        };

        self.pods.insert(pod_name.clone(), pod);
        self.pods_by_job
            .entry(job_label)
            .or_insert_with(HashSet::new)
            .insert(pod_name);

        Ok(())
    }

    fn remove(&mut self, pod: &k8s_openapi::api::core::v1::Pod) -> DiscoveryCacheResult<()> {
        use DiscoveryCacheError::*;

        let pod_name = pod
            .metadata
            .name
            .as_deref()
            .ok_or(Fatal("could not get pod name"))?
            .to_owned();

        let existing_pod = match self.pods.remove(&pod_name) {
            Some(pod) => pod,
            None => return Err(Ignored("pod not found in cache")),
        };

        if let Some(pod_set) = self.pods_by_job.get_mut(&existing_pod.job) {
            pod_set.remove(&pod_name);
            if pod_set.is_empty() {
                self.pods_by_job.remove(&existing_pod.job);
            }
        }

        Ok(())
    }

    fn report(
        &self,
        action: &str,
        pod: &k8s_openapi::api::core::v1::Pod,
        result: &DiscoveryCacheResult<()>,
    ) {
        let pod_name = pod.metadata.name.as_deref().unwrap_or("<unknown>");

        match result {
            Ok(_) => (),
            Err(DiscoveryCacheError::Ignored(reason)) => {
                log::debug!("{} ignored for pod {:?}: {}", action, pod_name, reason);
            }
            Err(DiscoveryCacheError::Fatal(reason)) => {
                log::error!("{} failed for pod {:?}: {}", action, pod_name, reason);
                panic!("fatal error in discovery cache");
            }
        }
    }
}

impl K8sCache for DiscoveryCache {
    type Resource = k8s_openapi::api::core::v1::Pod;

    fn reset(&mut self, list: ObjectList<Self::Resource>) {
        self.pods.clear();
        self.pods_by_job.clear();
        for pod in list.items {
            self.update(WatchEvent::Added(pod));
        }
    }

    fn update(&mut self, event: WatchEvent<Self::Resource>) {
        match event {
            WatchEvent::Added(o) => {
                let insert = self.insert(&o);
                self.report("insert", &o, &insert);
                if insert.is_ok() {
                    log::info!("pod added to discovery cache: {:?}", o.metadata.name);
                }
            }

            WatchEvent::Modified(o) => {
                let remove = self.remove(&o);
                self.report("remove", &o, &remove);
                let insert = self.insert(&o);
                self.report("insert", &o, &insert);

                let did_remove = remove.is_ok();
                let did_insert = insert.is_ok();
                match (did_remove, did_insert) {
                    (true, true) => {
                        log::info!("pod updated in discovery cache: {:?}", o.metadata.name)
                    }
                    (false, true) => {
                        log::info!("pod added to discovery cache: {:?}", o.metadata.name)
                    }
                    (true, false) => {
                        log::info!("pod removed from discovery cache: {:?}", o.metadata.name)
                    }
                    (false, false) => (),
                }
            }

            WatchEvent::Deleted(o) => {
                let remove = self.remove(&o);
                self.report("remove", &o, &remove);
                if remove.is_ok() {
                    log::info!("pod removed from discovery cache: {:?}", o.metadata.name);
                }
            }

            WatchEvent::Bookmark(o) => {
                log::debug!("bookmark: {:?} (noop)", o.metadata.resource_version);
            }
            WatchEvent::Error(e) => {
                log::error!("watch error: {:?}", e);
            }
        }
    }
}

struct K8sWatcher<T: K8sCache> {
    api: Api<T::Resource>,
    data: RwLock<K8sWatcherData<T>>,
}

struct K8sWatcherData<T: K8sCache> {
    resource_version: Option<String>,
    data: T,
}

struct K8sWatcherReadGuard<'a, T: K8sCache> {
    lock: tokio::sync::RwLockReadGuard<'a, K8sWatcherData<T>>,
}

impl<'a, T: K8sCache> Deref for K8sWatcherReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.lock.data
    }
}

impl<T: K8sCache> K8sWatcher<T>
where
    T::Resource: kube::Resource,
{
    async fn new(api: Api<T::Resource>, data: T) -> Arc<Self> {
        let inner = K8sWatcherData {
            resource_version: None,
            data,
        };
        Arc::new(K8sWatcher {
            api,
            data: RwLock::new(inner),
        })
    }

    fn start(self: &Arc<Self>) {
        let inner = Arc::downgrade(&self);

        tokio::spawn(async move {
            log::debug!("watcher task starting");

            while let Some(this) = inner.upgrade() {
                match this.try_init().await {
                    Ok(_) => {
                        log::debug!("successfully initialized k8s watcher");
                        break;
                    }
                    Err(e) => {
                        log::error!("failed to initialize k8s watcher: {}", e);
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }

            while let Some(this) = inner.upgrade() {
                match this.watch_iter().await {
                    Ok(_) => (),
                    Err(e) => log::error!("k8s watcher error: {}", e),
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }

            log::debug!("watcher task exiting");
        });
    }

    async fn read(&self) -> K8sWatcherReadGuard<'_, T> {
        let lock = self.data.read().await;
        K8sWatcherReadGuard { lock }
    }

    async fn try_init(&self) -> Result<(), kube::Error> {
        log::info!("initializing k8s watcher");

        let list = self.api.list(&Default::default()).await?;

        let resource_version = list
            .metadata
            .resource_version
            .clone()
            .expect("no resource version in list");

        let _ = {
            let mut lock = self.data.write().await;

            lock.resource_version = Some(resource_version);
            lock.data.reset(list);
        };

        Ok(())
    }

    async fn watch_iter(&self) -> Result<(), kube::Error> {
        let mut watch = {
            let lock = self.data.read().await;

            let resource_version = lock
                .resource_version
                .as_ref()
                .expect("no resource version in cache");

            log::debug!("starting k8s watch iteration from {:?}", resource_version);

            let watch = self
                .api
                .watch(&Default::default(), &resource_version)
                .await?;
            Box::pin(watch)
        };

        while let Some(event_result) = watch.next().await {
            let event = event_result?;

            let resource_version = {
                let resource_version = match &event {
                    WatchEvent::Added(ev) => ev.resource_version().clone(),
                    WatchEvent::Modified(ev) => ev.resource_version().clone(),
                    WatchEvent::Deleted(ev) => ev.resource_version().clone(),
                    WatchEvent::Bookmark(ev) => Some(ev.metadata.resource_version.clone()),
                    WatchEvent::Error(e) => {
                        log::warn!("watch error event: {:?}", e);
                        break;
                    }
                };
                resource_version
                    .clone()
                    .expect("no resource version in event")
            };

            let _ = {
                let mut lock = self.data.write().await;
                lock.resource_version = Some(resource_version);
                lock.data.update(event);
            };
        }

        Ok(())
    }
}
