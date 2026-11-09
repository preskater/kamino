//! Pagination primitives shared by every list endpoint.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

/// Common query parameters accepted by every collection endpoint.
#[derive(Debug, Default, Deserialize)]
pub struct ListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Column name to sort by; validated against the endpoint's whitelist.
    pub sort: Option<String>,
    /// `asc` or `desc`. Defaults to `desc` when sorting by the default column.
    pub order: Option<String>,
    /// Free text search across the endpoint's searchable columns.
    pub q: Option<String>,
    /// Resource specific exact-match filters, validated against a whitelist.
    #[serde(flatten)]
    pub filters: BTreeMap<String, String>,
}

impl ListParams {
    #[must_use]
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
    }

    #[must_use]
    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    pub fn with_filter(&mut self, key: &str, value: impl Into<String>) {
        self.filters.insert(key.to_string(), value.into());
    }
}

/// A page of results plus the total number of matching rows.
#[derive(Debug, Serialize, ToSchema)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl<T> Page<T> {
    #[must_use]
    pub fn new(items: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        Self {
            items,
            total,
            limit,
            offset,
        }
    }
}
