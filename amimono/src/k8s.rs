use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use futures::future::BoxFuture;
use rand::seq::IndexedRandom;
use tokio::sync::Mutex;

use crate::{
    config::Binding,
    runtime::{self, Location},
};

const CACHE_MIN: Duration = Duration::from_secs(1);
const CACHE_MAX: Duration = Duration::from_secs(3);

struct DiscoveryCacheEntry {
    locations: Vec<Location>,
    created: Instant,
    expiry: Duration,
}

impl DiscoveryCacheEntry {
    fn empty() -> Self {
        DiscoveryCacheEntry {
            locations: Vec::new(),
            created: Instant::now(),
            expiry: Duration::ZERO,
        }
    }

    fn new(locations: Vec<Location>) -> Self {
        let now = Instant::now();
        let expiry = rand::random_range(CACHE_MIN..CACHE_MAX);
        DiscoveryCacheEntry {
            locations,
            created: now,
            expiry,
        }
    }

    fn is_expired(&self) -> bool {
        self.created.elapsed() >= self.expiry
    }
}

pub struct K8sDiscovery {
    namespace: String,
    client: kube::Client,
    discovery_cache: Mutex<HashMap<String, DiscoveryCacheEntry>>,
}

impl K8sDiscovery {
    pub async fn new(namespace: String, config: kube::config::Config) -> Self {
        let client = kube::Client::try_from(config).expect("failed to create Kubernetes client");
        K8sDiscovery {
            namespace,
            client,
            discovery_cache: Mutex::new(HashMap::new()),
        }
    }

    async fn discover_real(&self, component: &str) -> DiscoveryCacheEntry {
        use k8s_openapi::api::core::v1::Pod;
        use kube::api::{Api, ListParams};

        let binding = runtime::binding_by_label(component);
        let job = runtime::config()
            .component_job(component)
            .expect("component not found");

        log::debug!("getting endpoints for {} job {}", component, job);
        let pods_api: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams::default().labels(&format!("amimono-job={}", component));
        let pods = pods_api.list(&lp).await.expect("TODO: handle error");

        let locations = pods
            .items
            .iter()
            .flat_map(|p| p.status.as_ref())
            .filter(|stat| stat.phase.as_deref() == Some("Running"))
            .filter_map(|stat| stat.pod_ip.as_ref())
            .map(|ip| match binding {
                Binding::Http(port) => {
                    let url = format!("http://{}:{}", ip, port);
                    Location::Http(url)
                }
                Binding::None => Location::None,
            })
            .collect::<Vec<_>>();

        DiscoveryCacheEntry::new(locations)
    }

    async fn discover_cached(&self, component: &str) -> Location {
        let mut cache = self.discovery_cache.lock().await;

        let entry = cache
            .entry(component.to_owned())
            .or_insert_with(|| DiscoveryCacheEntry::empty());
        if entry.is_expired() {
            *entry = self.discover_real(component).await;
        }

        entry
            .locations
            .choose(&mut rand::rng())
            .cloned()
            .unwrap_or(Location::None)
    }
}

impl runtime::DiscoveryProvider for K8sDiscovery {
    fn discover(&'_ self, component: &'static str) -> BoxFuture<'_, Location> {
        Box::pin(self.discover_cached(component))
    }
}
