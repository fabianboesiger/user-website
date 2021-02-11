use crate::{
    model::{update_session, with_session, Flashes, Session, Layout, User},
    Error,
};
use askama::Template;
use std::collections::HashMap;
use warp::{filters::BoxedFilter, http::Uri, Filter, Rejection, Reply};

#[derive(Template)]
#[template(path = "account.html")]
struct Account {
    _parent: Layout,
}

async fn get_account(session: Session) -> Result<(impl Reply, Session), Rejection> {
    let reply = warp::reply::html(
        Account {
            _parent: session.get_layout()
        }
        .render()
        .map_err(|err| Error::from(err))?,
    );

    Ok((reply, session))
}

async fn post_signout(
    mut session: Session,
) -> Result<(impl Reply, Session), Rejection> {
    session.unlink_user().await?;
    Ok((warp::redirect(Uri::from_static("/")), session))
}

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::path("account")
        .and(warp::path::end())
        .and(warp::get())
        .and(with_session())
        .and_then(get_account)
        .untuple_one()
        .and_then(update_session)
        .or(warp::path("signout")
            .and(warp::path::end())
            .and(warp::post())
            .and(with_session())
            .and_then(post_signout)
            .untuple_one()
            .and_then(update_session))
        .boxed()
}
