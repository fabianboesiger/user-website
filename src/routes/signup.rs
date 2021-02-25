use crate::{
    combine,
    model::{
        session::{update_session, with_session, Flashes, Layout, Session},
        user::{extract_confirm_password, extract_username, User},
    },
    Error,
};
use askama::Template;
use std::collections::HashMap;
use warp::{filters::BoxedFilter, http::Uri, Filter, Rejection, Reply};

#[derive(Template)]
#[template(path = "signup.html")]
struct Signup {
    _parent: Layout,
    flashes: Flashes,
}

async fn get_signup(mut session: Session) -> Result<(impl Reply, Session), Rejection> {
    let reply = warp::reply::html(
        Signup {
            _parent: session.get_layout(),
            flashes: session.get_flashes(),
        }
        .render()
        .map_err(|err| Error::from(err))?,
    );

    Ok((reply, session))
}

async fn post_signup(
    mut session: Session,
    mut form: HashMap<String, String>,
) -> Result<(impl Reply, Session), Rejection> {
    if let Some(user) = session.add_flashes(combine!(User {
        username: extract_username(&mut form),
        password: extract_confirm_password(&mut form),
    })) {
        if let Some(user) = session.add_flashes(user.signup().await?) {
            session.link_user(user).await?;
            return Ok((warp::redirect(Uri::from_static("/")), session));
        }
    }

    Ok((warp::redirect(Uri::from_static("/signup")), session))
}

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::path("signup")
        .and(warp::path::end())
        .and(
            warp::get()
                .and(with_session())
                .and_then(get_signup)
                .untuple_one()
                .and_then(update_session)
                .or(warp::post()
                    .and(with_session())
                    .and(warp::body::form())
                    .and_then(post_signup)
                    .untuple_one()
                    .and_then(update_session)),
        )
        .boxed()
}
