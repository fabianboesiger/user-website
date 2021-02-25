use crate::{
    model::session::{update_session, with_session, Layout, Session},
    Error,
};
use askama::Template;
use warp::ws::{Message, WebSocket};
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};
use futures::{StreamExt, SinkExt};
use log::*;
use crate::model::user::UserId;

#[derive(Template)]
#[template(path = "game.html")]
struct Game {
    _parent: Layout,
}

async fn get_game(session: Session) -> Result<(impl Reply, Session), Rejection> {
    let reply = warp::reply::html(
        Game {
            _parent: session.get_layout(),
        }
        .render()
        .map_err(|err| Error::from(err))?,
    );

    Ok((reply, session))
}

async fn upgrade_ws(
    session: Session,
    ws: warp::ws::Ws,
) -> Result<(impl Reply, Session), Rejection> {
    let user_id = session.get_user_id()?;
    
    Ok((
        ws.on_upgrade(move |socket| connect_ws(user_id, socket)),
        session,
    ))
}

// TODO: Proper error handling.
async fn connect_ws(user_id: UserId, websocket: WebSocket) {
    info!("new websocket connected");

    let (mut ws_tx, mut ws_rx) = websocket.split();
    let (tx, mut rx) = crate::game::message::CLIENT_CREATOR.get().unwrap().create();

    tokio::join!(
        // Send to client.
        async move {
            while let Some(Ok(message)) = ws_rx.next().await {
                if message.is_text() {
                    if let Ok(message) = String::from_utf8(message.into_bytes()) {
                        if let Ok(message) = serde_json::from_str(&message) {
                            tx.send((user_id, message)).await.ok();
                        }
                    }
                }
            }   
        },
        // Receive from client.
        async move {
            while let Ok((client, message)) = rx.recv().await {
                if client.includes(user_id) {
                    if let Ok(message) = serde_json::to_string(&message) {
                        ws_tx.send(Message::text(message)).await.ok();
                    }
                }
            }   
        }
    );
    
}

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::path("game")
        .and(
            warp::path::end()
                .and(warp::get())
                .and(with_session())
                .and_then(get_game)
                .untuple_one()
                .and_then(update_session)
                .or(warp::path("ws")
                .and(warp::path::end())
                .and(with_session())
                .and(warp::ws())
                .and_then(upgrade_ws)
                .untuple_one()
                .and_then(update_session))
        )
        .boxed()
}
