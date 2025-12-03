use std::collections::{BTreeMap, HashMap};

use futures::future::BoxFuture;

use crate::component::ComponentId;

/// The configuration for a single component.
pub struct ComponentConfig {
    /// An opaque identifier for this component's `Component` impl. This can
    /// be generated with `Component::id()`. A `Component` impl is necessary
    /// for accessing information such as bindings.
    pub id: ComponentId,

    /// This component's label, a string identifier. Every component must have a
    /// unique label. The label is mostly used for external things like logging
    /// and as a key in config files. Within the application, components are
    /// identified with a type that implements the `Component` trait.
    pub label: String,

    /// The ports this component will bind. This is metadata used for things
    /// like generating container configs. Components within the same job can
    /// have the same port numbers here, as long as they have a mechanism for
    /// sharing the port.
    pub ports: Vec<u16>,

    /// Indicates whether the component is stateful. Stateful components can use
    /// local storage that will be persisted across application revisions.
    pub is_stateful: bool,

    pub(crate) entry: fn(barrier: &'static tokio::sync::Barrier) -> BoxFuture<'static, ()>,
}

/// A fully configured application.
///
/// Refer to the [module-level documentation][crate::config] for more information.
pub struct AppConfig {
    revision: String,
    component_jobs: HashMap<String, String>,
    jobs: BTreeMap<String, JobConfig>,
}

impl AppConfig {
    /// The job the component is assigned to.
    pub fn component_job(&self, label: &str) -> Option<&str> {
        self.component_jobs.get(label).map(|s| s.as_str())
    }

    /// The application's revision identifier.
    pub fn revision(&self) -> &str {
        self.revision.as_str()
    }

    /// Retrieve a `JobConfig` by its label.
    pub fn job(&self, label: &str) -> Option<&JobConfig> {
        self.jobs.get(label)
    }

    /// An iterator over the `JobConfig`s in the application.
    pub fn jobs(&self) -> impl Iterator<Item = &JobConfig> {
        self.jobs.values()
    }

    /// Retrieve a `ComponentConfig` by its label.
    pub fn component(&self, label: &str) -> Option<&ComponentConfig> {
        let job_label = self.component_job(label)?;
        let job = self.job(job_label)?;
        job.component(label)
    }
}

/// A fully configured job.
///
/// Refer to the [module-level documentation][crate::config] for more information.
pub struct JobConfig {
    label: String,
    components: BTreeMap<String, ComponentConfig>,
}

impl JobConfig {
    /// An iterator over the `ComponentConfig`s in the job.
    pub fn components(&self) -> impl Iterator<Item = &ComponentConfig> {
        self.components.values()
    }

    /// Retrieve a `ComponentConfig` by its label.
    pub fn component(&self, label: &str) -> Option<&ComponentConfig> {
        self.components.get(label)
    }

    /// The job's label.
    pub fn label(&self) -> &str {
        self.label.as_str()
    }

    /// Indicates whether the job is stateful. A job is stateful if any of its
    /// components are stateful.
    pub fn is_stateful(&self) -> bool {
        self.components().any(|c| c.is_stateful)
    }
}

/// A helper for constructing a `JobConfig`.
///
/// Refer to the [module-level documentation][crate::config] for more information.
pub struct JobBuilder {
    label: Option<String>,
    components: BTreeMap<String, ComponentConfig>,
}

impl JobBuilder {
    /// Create an empty `JobBuilder`.
    pub fn new() -> JobBuilder {
        JobBuilder {
            label: None,
            components: BTreeMap::new(),
        }
    }

    /// Convert the builder into a `JobConfig`.
    pub fn build(&mut self) -> JobConfig {
        let comps = std::mem::take(&mut self.components);
        if comps.len() == 0 {
            panic!("jobs must have at least one component");
        }
        let label = match std::mem::take(&mut self.label) {
            Some(label) => label,
            None => {
                if comps.len() > 1 {
                    panic!("jobs with multiple components must have an explicit label")
                }
                comps.values().next().unwrap().label.clone()
            }
        };
        JobConfig {
            label,
            components: comps,
        }
    }

    pub fn install<F: FnOnce(&mut JobBuilder)>(&mut self, f: F) -> &mut JobBuilder {
        f(self);
        self
    }

    /// Set the job's label.
    pub fn with_label<S: Into<String>>(&mut self, label: S) -> &mut JobBuilder {
        self.label = Some(label.into());
        self
    }

    /// Add a component to the job.
    pub fn add_component<C: Into<ComponentConfig>>(&mut self, comp: C) -> &mut JobBuilder {
        let comp = comp.into();
        let key = comp.label.clone();
        if self.components.insert(key.clone(), comp).is_some() {
            panic!("duplicate component label: {}", key);
        }
        self
    }
}

impl From<&mut JobBuilder> for JobConfig {
    fn from(builder: &mut JobBuilder) -> Self {
        builder.build()
    }
}

impl From<ComponentConfig> for JobConfig {
    fn from(value: ComponentConfig) -> Self {
        JobBuilder::new().add_component(value).build()
    }
}

impl From<&mut AppBuilder> for AppConfig {
    fn from(builder: &mut AppBuilder) -> Self {
        builder.build()
    }
}

/// A helper for constructing an `AppConfig`.
///
/// Refer to the [module-level documentation][crate::config] for more information.
pub struct AppBuilder {
    app: AppConfig,
}

impl AppBuilder {
    /// Create an empty `AppBuilder`.
    pub fn new(revision: &str) -> AppBuilder {
        AppBuilder {
            app: AppConfig {
                revision: revision.to_owned(),
                component_jobs: HashMap::new(),
                jobs: BTreeMap::new(),
            },
        }
    }

    /// Convert the builder into an `AppConfig`.
    pub fn build(&mut self) -> AppConfig {
        AppConfig {
            revision: self.app.revision.clone(),
            component_jobs: std::mem::take(&mut self.app.component_jobs),
            jobs: std::mem::take(&mut self.app.jobs),
        }
    }

    pub fn install<F: FnOnce(&mut AppBuilder)>(&mut self, f: F) -> &mut AppBuilder {
        f(self);
        self
    }

    /// Add a job to the app.
    pub fn add_job<J: Into<JobConfig>>(&mut self, job: J) -> &mut AppBuilder {
        let job = job.into();
        let label = job.label.clone();
        for comp in job.components() {
            let comp_label = comp.label.clone();
            let current_job = self
                .app
                .component_jobs
                .insert(comp.label.clone(), label.clone());
            if let Some(other_label) = current_job {
                panic!(
                    "component {} already assigned to job {}",
                    comp_label, other_label
                );
            }
        }
        if self.app.jobs.insert(label.clone(), job).is_some() {
            panic!("duplicate job label: {}", label);
        }
        self
    }
}
