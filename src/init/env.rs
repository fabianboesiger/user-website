macro_rules! static_env {
    ( $( $i:ident : $t:ty ),* $(,)? ) => {
        $(
            pub static $i: once_cell::sync::Lazy<$t> = once_cell::sync::Lazy::new(|| std::env::var(stringify!($i)).unwrap().parse::<$t>().unwrap());
        )*
    };
}

static_env! {
    DATABASE_URL: String,
    PORT: u16,
}
