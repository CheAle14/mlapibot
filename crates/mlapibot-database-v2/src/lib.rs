#![allow(async_fn_in_trait)]
#![feature(string_replace_in_place)]
pub mod client;
pub mod errors;
pub mod migrations;
pub mod repos;

pub(crate) type DateTimeUtc = chrono::DateTime<chrono::Utc>;
