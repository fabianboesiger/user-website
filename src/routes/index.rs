use crate::{
    model::session::{update_session, with_session, Layout, Session},
    Error,
};
use askama::Template;
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    _parent: Layout,
}

async fn get_index(session: Session) -> Result<(impl Reply, Session), Rejection> {
    let reply = warp::reply::html(
        Index {
            _parent: session.get_layout(),
        }
        .render()
        .map_err(|err| Error::from(err))?,
    );

    Ok((reply, session))
}

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    warp::path::end()
        .and(warp::get())
        .and(with_session())
        .and_then(get_index)
        .untuple_one()
        .and_then(update_session)
        .boxed()
}
