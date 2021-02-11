pub mod database;
pub mod env;
pub mod regexes;

pub async fn init() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    database::init().await;
}
