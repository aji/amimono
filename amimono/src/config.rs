//! Types and other definitions for defining Amimono applications.
//!
//! It's important that [`AppConfig`]s be constructed deterministically so that
//! all workloads can use the same value. In the future this may be encouraged
//! by allowing configuration code to be written with `const fn`s, but this is
//! not currently implemented.
//!
//! # Example
//!
//! Most config values are constructed with builders such as [`AppBuilder`] and
//! [`JobBuilder`], however [`ComponentConfig`] is meant to be constructed
//! directly. The suggested way to organize your configuration code is to have
//! one function per component that returns a [`ComponentConfig`], and then a
//! single function close to your `main()` function that assembles these into
//! an [`AppConfig`]. This might look as follows:
//!
//! ```
//! use amimono::config::{AppBuilder, AppConfig, JobBuilder};
//!
//! use crate::backend;
//! use crate::frontend;
//!
//! pub fn configure() -> AppConfig {
//!     AppBuilder::new()
//!         .add_job(
//!             JobBuilder::new()
//!                 .with_label("backend")
//!                 .add_component(backend::emailer::component())
//!                 .add_component(backend::accounts::component())
//!                 .add_component(backend::images::component())
//!                 .add_component(backend::orders::component())
//!                 .build()
//!         )
//!         .add_job(
//!             JobBuilder::new()
//!                 .with_label("cache")
//!                 .add_component(backend::cache::component())
//!                 .build()
//!         )
//!         .add_job(
//!             JobBuilder::new()
//!                 .with_label("frontend")
//!                 .add_component(frontend::component())
//!                 .build()
//!         )
//!         .build()
//! }
//! ```
//!
//! However you are free to organize things in whatever way you prefer.

use std::collections::{BTreeMap, HashMap};

use crate::runtime::ComponentId;

/// A request for a type of binding.
#[derive(Copy, Clone, Debug)]
pub enum BindingType {
    None,
    Http,
    HttpFixed(u16),
}

/// An allocated binding.
#[derive(Clone, Debug)]
pub enum Binding {
    None,
    Http(u16),
}

/// The configuration for a single component.
pub struct ComponentConfig {
    /// This component's label, a string identifier. Every component must have a
    /// unique label. The label is mostly used for external things like logging
    /// and as a key in config files. Within the application, components are
    /// identified with a type that implements the `Component` trait.
    pub label: String,

    /// An opaque identifier for this component's `Component` impl. This can
    /// be generated with `Component::id()`. A `Component` impl is necessary
    /// for accessing information such as bindings.
    pub id: ComponentId,

    /// The binding type requested by this component. When the component starts,
    /// the allocated binding can be accessed with
    /// [`runtime::binding`](crate::runtime::binding)
    pub binding: BindingType,

    /// The component's entry point.
    pub entry: fn(),
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
    pub fn component_job(&self, label: &str) -> Option<&str> {
        self.component_jobs.get(label).map(|s| s.as_str())
    }

    pub fn revision(&self) -> &str {
        self.revision.as_str()
    }

    /// A function to retrieve a `JobConfig` by its label.
    pub fn job(&self, label: &str) -> Option<&JobConfig> {
        self.jobs.get(label)
    }

    /// An iterator over the `JobConfig`s in the application.
    pub fn jobs(&self) -> impl Iterator<Item = &JobConfig> {
        self.jobs.values()
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

    /// The job's label.
    pub fn label(&self) -> &str {
        self.label.as_str()
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
    pub fn build(self) -> JobConfig {
        let comps = self.components;
        if comps.len() == 0 {
            panic!("jobs must have at least one component");
        }
        let label = match self.label {
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

    /// Set the job's label.
    pub fn with_label<S: Into<String>>(mut self, label: S) -> JobBuilder {
        self.label = Some(label.into());
        self
    }

    /// Add a component to the job.
    pub fn add_component<C: Into<ComponentConfig>>(mut self, comp: C) -> JobBuilder {
        let comp = comp.into();
        let key = comp.label.clone();
        if self.components.insert(key.clone(), comp).is_some() {
            panic!("duplicate component label: {}", key);
        }
        self
    }
}

impl From<JobBuilder> for JobConfig {
    fn from(builder: JobBuilder) -> Self {
        builder.build()
    }
}

impl From<ComponentConfig> for JobConfig {
    fn from(value: ComponentConfig) -> Self {
        JobBuilder::new().add_component(value).build()
    }
}

impl From<AppBuilder> for AppConfig {
    fn from(builder: AppBuilder) -> Self {
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
    pub fn build(self) -> AppConfig {
        self.app
    }

    /// Add a job to the app.
    pub fn add_job<J: Into<JobConfig>>(mut self, job: J) -> AppBuilder {
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
