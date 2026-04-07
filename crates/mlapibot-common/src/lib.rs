macro_rules! moduse {
    ($(
        $name:ident
    ),* $(,)?) => {
        $(
            mod $name;
            pub use $name::*;
        )*
    };
}

moduse!(
    detected_item,
    detection,
    words,
    needle_finder,
    lowercase,
    cached,
    errors,
    running_stat,
);

pub mod action;
pub mod config;
pub mod matchers;
