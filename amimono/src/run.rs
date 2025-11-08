use crate::{
    Application,
    local::{LocalConfigBuilder, LocalLauncher},
};

pub fn run<A: Application>(app: A) {
    let mut launcher = {
        let mut builder = LocalConfigBuilder::new();
        app.setup(&mut builder);
        LocalLauncher::new(builder.build())
    };
    app.setup(&mut launcher);
    launcher.finish();
}
