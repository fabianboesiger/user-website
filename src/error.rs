use askama::Template;
use std::convert::Infallible;
use warp::{
    http::StatusCode,
    {reject::Reject, Rejection, Reply},
};

#[derive(Debug)]
pub enum Error {
    Unauthorized,
    Database(sqlx::Error),
    Template(askama::Error),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Error {
        Error::Database(err)
    }
}

impl From<askama::Error> for Error {
    fn from(err: askama::Error) -> Error {
        Error::Template(err)
    }
}

impl Reject for Error {}
/*
impl Into<Rejection> for Error {
    fn into(self) -> Rejection {
        warp::reject::custom(self)
    }
}
*/
#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundTemplate;

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    code: StatusCode,
    details: String,
}

pub async fn handle_rejection(rejection: Rejection) -> Result<impl Reply, Infallible> {
    if rejection.is_not_found() {
        Ok(warp::reply::with_status(
            warp::reply::html(NotFoundTemplate.render().unwrap()),
            StatusCode::NOT_FOUND,
        ))
    } else {
        let code = StatusCode::INTERNAL_SERVER_ERROR;
        Ok(warp::reply::with_status(
            warp::reply::html(
                ErrorTemplate {
                    code,
                    details: format!("{:#?}", rejection),
                }
                .render()
                .unwrap(),
            ),
            code,
        ))
    }
}
