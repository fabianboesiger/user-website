use once_cell::sync::OnceCell;
use tokio::sync::{mpsc, broadcast};
use serde::{Deserialize, Serialize};
use crate::model::user::UserId;

pub static CLIENT_CREATOR: OnceCell<ClientCreator> = OnceCell::new();

pub type ServerEndpont = (&'static broadcast::Sender<(Client, ServerMessage)>, mpsc::Receiver<(UserId, ClientMessage)>);
pub type ClientEndpoint = (mpsc::Sender<(UserId, ClientMessage)>, broadcast::Receiver<(Client, ServerMessage)>);

#[derive(Debug)]
pub struct ClientCreator(mpsc::Sender<(UserId, ClientMessage)>, &'static broadcast::Sender<(Client, ServerMessage)>);

impl ClientCreator {
    pub fn init() -> ServerEndpont {
        let (client_tx, rx) = mpsc::channel(16);
        let (tx, _) = broadcast::channel(16);

        let leaked_tx: &'static broadcast::Sender<(Client, ServerMessage)> = Box::leak(Box::new(tx));

        CLIENT_CREATOR.set(ClientCreator(client_tx, leaked_tx)).unwrap();

        (leaked_tx, rx)
    }

    pub fn create(&self) -> ClientEndpoint {
        (self.0.clone(), self.1.subscribe())
    }
}

#[derive(Clone)]
pub enum Client {
    User(UserId),
    All,
}

impl Client {
    pub fn includes(&self, user_id: UserId) -> bool {
        match self {
            Self::All => true,
            Self::User(client) => *client == user_id
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ScoreboardEntry {
    pub username: Option<String>,
    pub score: Option<i32>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    UpdateScore(i32),
    UpdateScoreboard(Vec<ScoreboardEntry>),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Increment,
    Init,
}