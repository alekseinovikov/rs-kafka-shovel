extern crate core;

use tracing::info;

mod transformer;
mod topic_processor;

fn main() {
    tracing_subscriber::fmt::init();

    info!("Hello, world!")
}
