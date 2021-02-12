use crate::{
    model::{
        session::{
            update_session,
            with_session,
            Flashes,
            Session,
            Layout
        },
        user::{extract_username, update_username, extract_confirm_password, update_password, extract_password, User},
    },
    Error,
};
use askama::Template;
use std::collections::HashMap;
use warp::{filters::BoxedFilter, http::Uri, Filter, Rejection, Reply};

#[derive(Template)]
#[template(path = "account.html")]
struct Account {
    _parent: Layout,
    flashes: Flashes,
    username: String,
}

async fn get_account(mut session: Session) -> Result<(impl Reply, Session), Rejection> {
    let reply = warp::reply::html(
        Account {
            _parent: session.get_layout(),
            flashes: session.get_flashes(),
            username: session.get_username()?,
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

async fn post_username(
    mut session: Session,
    mut form: HashMap<String, String>,
) -> Result<(impl Reply, Session), Rejection> {
    let old = session.get_username()?;

    if let Some(new) = session.add_flashes(
        extract_username(&mut form)
    ) {
        update_username(old, new).await?;
    }
    
    Ok((warp::redirect(Uri::from_static("/account")), session))
}

async fn post_password(
    mut session: Session,
    mut form: HashMap<String, String>,
) -> Result<(impl Reply, Session), Rejection> {
    let username = session.get_username()?;

    if let Some(password) = session.add_flashes(
        extract_confirm_password(&mut form)
    ) {
        update_password(username, password).await?;
    }
    
    Ok((warp::redirect(Uri::from_static("/account")), session))
}

async fn post_delete(
    mut session: Session,
    mut form: HashMap<String, String>,
) -> Result<(impl Reply, Session), Rejection> {
    let username = session.get_username()?;

    if let Some(password) = session.add_flashes(
        extract_password(&mut form)
    ) {
        if let Some(user) = session.add_flashes(User {username, password}.signin().await?) {
            session.unlink_user().await?;
            user.delete().await?;
            return Ok((warp::redirect(Uri::from_static("/")), session));
        }
    }
    
    Ok((warp::redirect(Uri::from_static("/account")), session))
}

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::path("account")
        .and(
            warp::path::end()
                .and(warp::get())
                .and(with_session())
                .and_then(get_account)
                .untuple_one()
                .and_then(update_session)
                .or(warp::path("username")
                    .and(warp::path::end())
                    .and(warp::post())
                    .and(with_session())
                    .and(warp::body::form())
                    .and_then(post_username)
                    .untuple_one()
                    .and_then(update_session))
                    .or(warp::path("password")
                        .and(warp::path::end())
                        .and(warp::post())
                        .and(with_session())
                        .and(warp::body::form())
                        .and_then(post_password)
                        .untuple_one()
                        .and_then(update_session))
                        .or(warp::path("delete")
                            .and(warp::path::end())
                            .and(warp::post())
                            .and(with_session())
                            .and(warp::body::form())
                            .and_then(post_delete)
                            .untuple_one()
                            .and_then(update_session)),
        )
        .or(warp::path("signout")
            .and(warp::path::end())
            .and(warp::post())
            .and(with_session())
            .and_then(post_signout)
            .untuple_one()
            .and_then(update_session))
        .boxed()
}
