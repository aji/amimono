use std::collections::HashMap;

use crate::{Component, Label};

pub struct AppConfig {
    comp_placement: HashMap<Label, Label>,
    jobs: HashMap<Label, JobConfig>,
}

impl AppConfig {
    fn new() -> AppConfig {
        AppConfig {
            comp_placement: HashMap::new(),
            jobs: HashMap::new(),
        }
    }

    fn add_job(&mut self, job: JobConfig) {
        for comp in job.components.iter() {
            if self
                .comp_placement
                .insert(comp.label(), job.label)
                .is_some()
            {
                panic!(
                    "component {} cannot be placed in multiple jobs",
                    comp.label()
                );
            }
        }

        if let Some(j) = self.jobs.insert(job.label, job) {
            panic!("cannot reuse job label {}", j.label);
        }
    }
}

pub struct AppBuilder {
    cf: AppConfig,
}

impl AppBuilder {
    pub fn new() -> AppBuilder {
        AppBuilder {
            cf: AppConfig::new(),
        }
    }

    pub fn build(self) -> AppConfig {
        self.cf
    }

    pub fn add_job<J: Into<JobConfig>>(mut self, job: J) -> AppBuilder {
        self.cf.add_job(job.into());
        self
    }
}

pub struct JobConfig {
    label: Label,
    replicas: usize,
    components: Vec<Box<dyn Component>>,
}

pub struct JobBuilder {
    label: Option<Label>,
    replicas: usize,
    components: Vec<Box<dyn Component>>,
}

impl JobBuilder {
    pub fn new() -> JobBuilder {
        JobBuilder {
            label: None,
            replicas: 1,
            components: Vec::new(),
        }
    }

    pub fn build(self) -> JobConfig {
        let label = match self.label {
            Some(x) => x,
            None => {
                if self.components.len() == 1 {
                    self.components[0].label()
                } else {
                    panic!("jobs with multiple components require an explicit label");
                }
            }
        };
        JobConfig {
            label,
            replicas: self.replicas,
            components: self.components,
        }
    }

    pub fn with_label(mut self, label: Label) -> JobBuilder {
        self.label = Some(label);
        self
    }

    pub fn with_replicas(mut self, n: usize) -> JobBuilder {
        assert!(n > 0, "number of replicas must be nonzero");
        self.replicas = n;
        self
    }

    pub fn add_component<C: Component>(mut self, comp: C) -> JobBuilder {
        self.components.push(Box::new(comp));
        self
    }
}

impl Into<JobConfig> for JobBuilder {
    fn into(self) -> JobConfig {
        self.build()
    }
}
