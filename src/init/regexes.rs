macro_rules! static_regex {
    ( $( $i:ident : $l:literal ),* $(,)? ) => {
        $(
            pub static $i: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| regex::Regex::new($l).unwrap());
        )*
    };
}

static_regex! {
    USERNAME_CHARS: r"^[a-zA-Z0-9_]*$",
    USERNAME_LENGTH: r"^.{2,16}$",
    PASSWORD_LENGTH: r"^.{4,32}$",
}
