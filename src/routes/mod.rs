mod index;
mod signup;
mod signin;
mod account;

use warp::{Filter, filters::BoxedFilter, Reply};

pub fn serve() -> BoxedFilter<(impl Reply,)> {
    index::serve()
        .or(signup::serve())
        .or(signin::serve())
        .or(account::serve())
        .boxed()
}
