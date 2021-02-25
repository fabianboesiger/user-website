mod event;
pub mod message;

use crate::{database::get_pool, Error};
use event::EventQueue;
use message::*;
use log::*;

pub async fn run((tx, mut rx): ServerEndpont) {
    let queue = EventQueue::new();
    while let Some((user_id, message)) = rx.recv().await {
        match message {
            ClientMessage::Increment => {
                let score = sqlx::query!(
                    "UPDATE states
                    SET score = score + 1
                    WHERE user_id = $1
                    RETURNING score",
                    user_id,
                )
                .fetch_one(get_pool())
                .await
                .unwrap()
                .score;

                let scoreboard = sqlx::query_as!(
                    ScoreboardEntry,
                    "SELECT username, score
                    FROM states
                    NATURAL JOIN users
                    ORDER BY score DESC",
                )
                .fetch_all(get_pool())
                .await
                .unwrap();

                tx.send((Client::User(user_id), ServerMessage::UpdateScore(score))).ok();
                tx.send((Client::All, ServerMessage::UpdateScoreboard(scoreboard))).ok();

            },
            ClientMessage::Init => {
                let score = sqlx::query!(
                    "SELECT score
                    FROM states
                    WHERE user_id = $1",
                    user_id,
                )
                .fetch_one(get_pool())
                .await
                .unwrap()
                .score;

                let scoreboard = sqlx::query_as!(
                    ScoreboardEntry,
                    "SELECT username, score
                    FROM states
                    NATURAL JOIN users
                    ORDER BY score DESC",
                )
                .fetch_all(get_pool())
                .await
                .unwrap();

                tx.send((Client::User(user_id), ServerMessage::UpdateScore(score))).ok();
                tx.send((Client::All, ServerMessage::UpdateScoreboard(scoreboard))).ok();
            }
        }
    }
}    