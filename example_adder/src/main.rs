pub mod adder;
pub mod app;
pub mod doubler;
pub mod driver;

fn main() {
    env_logger::init();
    amimono::run(app::configure());
}
