mod error;
mod init;
mod model;
mod routes;

pub use error::Error;
pub use init::*;

use env::PORT;
use warp::Filter;

#[tokio::main]
async fn main() {
    init().await;

    let routes = routes::serve()
        .or(warp::fs::dir("public"))
        .recover(error::handle_rejection)
        .with(warp::log("server"));

    warp::serve(routes).run(([127, 0, 0, 1], *PORT)).await;
}
