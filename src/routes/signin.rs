use crate::{
    combine,
    model::{
        session::{update_session, with_session, Flashes, Layout, Session},
        user::{extract_password, extract_username, User},
    },
    Error,
};
use askama::Template;
use std::collections::HashMap;
use warp::{filters::BoxedFilter, http::Uri, Filter, Rejection, Reply};

#[derive(Template)]
#[template(path = "signin.html")]
struct Signin {
    _parent: Layout,
    flashes: Flashes,
}

async fn get_signin(mut session: Session) -> Result<(impl Reply, Session), Rejection> {
    let reply = warp::reply::html(
        Signin {
            _parent: session.get_layout(),
            flashes: session.get_flashes(),
        }
        .render()
        .map_err(|err| Error::from(err))?,
    );

    Ok((reply, session))
}

async fn post_signin(
    mut session: Session,
    mut form: HashMap<String, String>,
) -> Result<(impl Reply, Session), Rejection> {
    if let Some(user) = session.add_flashes(combine!(User {
        username: extract_username(&mut form),
        password: extract_password(&mut form),
    })) {
        if let Some(user) = session.add_flashes(user.signin().await?) {
            session.link_user(user).await?;
            return Ok((warp::redirect(Uri::from_static("/")), session));
        }
    }

    Ok((warp::redirect(Uri::from_static("/signin")), session))
}

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::path("signin")
        .and(warp::path::end())
        .and(
            warp::get()
                .and(with_session())
                .and_then(get_signin)
                .untuple_one()
                .and_then(update_session)
                .or(warp::post()
                    .and(with_session())
                    .and(warp::body::form())
                    .and_then(post_signin)
                    .untuple_one()
                    .and_then(update_session)),
        )
        .boxed()
}
