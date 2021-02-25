use super::user::User;
use crate::{database::get_pool, error::Error};
use askama::Template;
use chrono::{DateTime, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{fmt::Display, iter};
use urlencoding::{decode, encode};
use warp::{
    http, reject, {Filter, Rejection, Reply},
};

pub type Flashes = Vec<String>;

#[derive(Template, Copy, Clone)]
#[template(path = "layout.html")]
pub struct Layout {
    pub signed_in: bool,
}

pub struct Session {
    layout: Layout,
    cookie: Cookie,
    flashes: Flashes,
}

impl Session {
    pub fn get_user_id(&self) -> Result<i32, Error> {
        self.cookie
            .user_id
            .ok_or(Error::Unauthorized)
    }

    pub async fn get_user(&self) -> Result<User, Error> {
        sqlx::query_as!(
            User,
            "SELECT username, password
            FROM users
            WHERE user_id = $1",
            self.cookie.user_id
        )
        .fetch_all(get_pool())
        .await?
        .pop()
        .ok_or(Error::Unauthorized)
    }
    
    pub fn get_layout(&self) -> Layout {
        self.layout
    }

    pub fn add_flashes<T, D: Display>(&mut self, result: Result<T, Vec<D>>) -> Option<T> {
        match result {
            Ok(t) => Some(t),
            Err(flashes) => {
                for flash in flashes {
                    self.flashes.push(format!("{}", flash));
                }
                None
            }
        }
    }

    pub fn get_flashes(&mut self) -> Vec<String> {
        self.flashes.drain(..).collect()
    }

    pub async fn link_user(&mut self, user: User) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE sessions
            SET user_id = (
                SELECT user_id
                FROM users
                WHERE username = $2
            )
            WHERE session_id = $1",
            self.cookie.session_id,
            user.username,
        )
        .execute(get_pool())
        .await?;

        Ok(())
    }

    pub async fn unlink_user(&mut self) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE sessions
            SET user_id = NULL
            WHERE session_id = $1",
            self.cookie.session_id,
        )
        .execute(get_pool())
        .await?;

        Ok(())
    }

    
    pub async fn update_username(&self, username: String) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE users
            SET username = $1
            WHERE user_id = $2",
            username,
            self.cookie.user_id,
        )
        .execute(get_pool())
        .await?;

        Ok(())
    }

    pub async fn update_password(&self, password: String) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE users
            SET password = $1
            WHERE user_id = $2",
            super::user::hash(password).await,
            self.cookie.user_id,
        )
        .execute(get_pool())
        .await?;

        Ok(())
    }
}

struct Cookie {
    session_id: String,
    user_id: Option<i32>,
    expires: DateTime<Utc>,
}

impl Cookie {
    fn random_id() -> String {
        let mut rng = thread_rng();
        iter::repeat(())
            .map(|_| rng.sample(Alphanumeric))
            .take(32)
            .map(|x| x as char)
            .collect()
    }

    pub async fn from_id(id: Option<String>) -> Result<Cookie, Error> {
        let cookie = if let Some(id) = id {
            sqlx::query_as!(
                Cookie,
                "SELECT *
                FROM sessions
                WHERE session_id = $1
                AND expires > NOW()",
                id
            )
            .fetch_all(get_pool())
            .await?
            .pop()
        } else {
            None
        };

        let cookie = if let Some(cookie) = cookie {
            log::info!("session {} connected", cookie.session_id);

            cookie
        } else {
            let cookie = sqlx::query_as!(
                Cookie,
                "INSERT INTO sessions (session_id)
                VALUES ($1)
                RETURNING *",
                Cookie::random_id(),
            )
            .fetch_one(get_pool())
            .await?;

            log::info!("new session {} created", cookie.session_id);

            cookie
        };

        Ok(cookie)
    }
}

pub fn with_session() -> impl Filter<Extract = (Session,), Error = Rejection> + Clone {
    warp::any()
        .and(warp::cookie::optional::<String>("session-id"))
        .and(warp::cookie::optional::<String>("flashes"))
        .and_then(|id: Option<String>, flashes: Option<String>| async move {
            match Cookie::from_id(id).await {
                Ok(cookie) => Ok((Session {
                    layout: Layout {
                        signed_in: cookie.user_id.is_some(),
                    },
                    cookie,
                    flashes: flashes
                        // TODO: Proper error handling.
                        .map(|string| {
                            decode(&string)
                                .unwrap_or(String::new())
                                .split('|')
                                .map(|flash| flash.to_owned())
                                .collect()
                        })
                        .unwrap_or(Vec::new()),
                },)),
                Err(err) => Err(reject::custom(err)),
            }
        })
        .untuple_one()
}

pub async fn update_session(
    reply: impl Reply,
    mut session: Session,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let reply = warp::reply::with_header(
        reply,
        http::header::SET_COOKIE,
        format!(
            "session-id={}; Path=/; Max-Age={}; HttpOnly",
            session.cookie.session_id,
            session
                .cookie
                .expires
                .signed_duration_since(Utc::now())
                .to_std()
                .unwrap()
                .as_secs(),
        ),
    );

    let reply = warp::reply::with_header(
        reply,
        http::header::SET_COOKIE,
        format!(
            "flashes={}; Path=/; Max-Age={}; HttpOnly",
            encode(&session.flashes.join("|")),
            if session.flashes.is_empty() { 0 } else { 60 }
        ),
    );

    session.flashes.clear();

    Ok(reply)
}
