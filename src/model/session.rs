use super::user::User;
use crate::{database::get_pool, error::Error};
use chrono::{DateTime, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{fmt::Display, iter};
use warp::{
    http, reject, {Filter, Rejection, Reply},
};
use urlencoding::{encode, decode};
use askama::Template;

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
    pub fn get_username(&self) -> Result<String, Error> {
        self.cookie.username.as_ref().cloned().ok_or(Error::Unauthorized)
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
        self.cookie.username = Some(user.username);
        self.update_user().await
    }

    pub async fn unlink_user(&mut self) -> Result<(), Error> {
        self.cookie.username = None;
        self.update_user().await
    }

    async fn update_user(&mut self) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE sessions
            SET username = $1
            WHERE id = $2",
            self.cookie.username,
            self.cookie.id,
        )
        .execute(get_pool())
        .await?;

        Ok(())
    }
}

struct Cookie {
    id: String,
    username: Option<String>,
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
                WHERE id = $1
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
            log::info!("session {} connected", cookie.id);

            cookie
        } else {
            let cookie = sqlx::query_as!(
                Cookie,
                "INSERT INTO sessions (id)
                VALUES ($1)
                RETURNING *",
                Cookie::random_id(),
            )
            .fetch_one(get_pool())
            .await?;

            log::info!("new session {} created", cookie.id);

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
                        signed_in: cookie.username.is_some(),
                    },
                    cookie,
                    flashes: flashes
                        // TODO: Proper error handling.
                        .map(|string| decode(&string).unwrap_or(String::new()).split('|').map(|flash| flash.to_owned()).collect())
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
            session.cookie.id,
            session.cookie.expires.signed_duration_since(Utc::now()).to_std().unwrap().as_secs(),
        ),
    );

    let reply = warp::reply::with_header(
        reply,
        http::header::SET_COOKIE,
        format!(
            "flashes={}; Path=/; Max-Age={}; HttpOnly",
            encode(&session.flashes.join("|")),
            if session.flashes.is_empty() {
                0
            } else {
                60
            }
        ),
    );

    session.flashes.clear();

    Ok(reply)
}
