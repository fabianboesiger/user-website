mod account;
mod game;
mod index;
mod signin;
mod signup;

use warp::{filters::BoxedFilter, Filter, Reply};

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    index::serve()
        .or(signup::serve())
        .or(signin::serve())
        .or(account::serve())
        .or(game::serve())
        .boxed()
}
