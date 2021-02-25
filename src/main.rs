#![feature(vecdeque_binary_search)]
#![feature(duration_constants)]

mod error;
mod game;
mod init;
mod model;
mod routes;

pub use error::Error;
pub use init::*;

use env::PORT;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Error> {
    init().await;

    let server_endpoint = game::message::ClientCreator::init();

    let routes = routes::serve()
        .or(warp::fs::dir("public"))
        .recover(error::handle_rejection)
        .with(warp::log("server"));

    tokio::join!(
        warp::serve(routes).run(([127, 0, 0, 1], *PORT)),
        game::run(server_endpoint),
    );

    Ok(())
}
