use std::net::SocketAddr;

use crate::runtime::ComponentRegistry;

#[derive(Copy, Clone, Debug)]
pub enum BindingType {
    None,
    Http,
}

#[derive(Clone)]
pub enum Binding {
    None,
    Http(SocketAddr, String),
}

pub struct ComponentConfig {
    pub label: String,
    pub binding: BindingType,
    pub register: fn(&mut ComponentRegistry, String),
    pub entry: fn(),
}

pub struct AppConfig {
    jobs: Vec<JobConfig>,
}

impl AppConfig {
    pub fn jobs(&self) -> impl Iterator<Item = &JobConfig> {
        self.jobs.iter()
    }
}

pub struct JobConfig {
    label: String,
    components: Vec<ComponentConfig>,
}

impl JobConfig {
    pub fn components(&self) -> impl Iterator<Item = &ComponentConfig> {
        self.components.iter()
    }

    pub fn label(&self) -> &str {
        self.label.as_str()
    }
}

pub struct JobBuilder {
    label: Option<String>,
    components: Vec<ComponentConfig>,
}

impl JobBuilder {
    pub fn new() -> JobBuilder {
        JobBuilder {
            label: None,
            components: Vec::new(),
        }
    }

    pub fn build(self) -> JobConfig {
        let label = match self.label {
            Some(label) => label,
            None => {
                if self.components.len() == 1 {
                    self.components[0].label.clone()
                } else {
                    panic!("jobs with multiple components must have an explicit label")
                }
            }
        };
        JobConfig {
            label,
            components: self.components,
        }
    }

    pub fn with_label<S: Into<String>>(mut self, label: S) -> JobBuilder {
        self.label = Some(label.into());
        self
    }

    pub fn add_component<C: Into<ComponentConfig>>(mut self, comp: C) -> JobBuilder {
        self.components.push(comp.into());
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

pub struct AppBuilder {
    app: AppConfig,
}

impl AppBuilder {
    pub fn new() -> AppBuilder {
        AppBuilder {
            app: AppConfig { jobs: Vec::new() },
        }
    }

    pub fn build(self) -> AppConfig {
        self.app
    }

    pub fn add_job<J: Into<JobConfig>>(mut self, job: J) -> AppBuilder {
        self.app.jobs.push(job.into());
        self
    }
}
