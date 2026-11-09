//! Pagination models and the runtime-checked SQL query layer.

pub mod pagination;
pub mod sql;

pub use pagination::{ListParams, Page};
pub use sql::{
    ListSpec, RawPage, Resource, SqlType, delete, get, get_as, insert, insert_as, list, list_as,
    prepare_create, to_map, update, update_as,
};
