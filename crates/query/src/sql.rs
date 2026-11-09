//! A thin, runtime-checked query layer.
//!
//! Every resource describes its table and column types once. Inserts, patches,
//! lookups and paginated lists are then built from that description, so the
//! 19 resources in this workspace share one implementation instead of 19
//! hand-written copies.
//!
//! Values are bound as text and cast in SQL (`$1::text::uuid`) which keeps the
//! binding code uniform without relying on compile-time query macros.

use serde::Serialize;
use serde_json::{Map, Value};
use sqlx::postgres::PgRow;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use kamino_error::{ApiResult, AppError};

use crate::pagination::{ListParams, Page};

/// Postgres types the runtime query layer knows how to cast from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlType {
    Text,
    Uuid,
    Bool,
    Int,
    BigInt,
    Numeric,
    Timestamptz,
    Date,
    Jsonb,
}

impl SqlType {
    #[must_use]
    pub fn cast(self) -> &'static str {
        match self {
            SqlType::Text => "text",
            SqlType::Uuid => "uuid",
            SqlType::Bool => "boolean",
            SqlType::Int => "integer",
            SqlType::BigInt => "bigint",
            SqlType::Numeric => "numeric",
            SqlType::Timestamptz => "timestamptz",
            SqlType::Date => "date",
            SqlType::Jsonb => "jsonb",
        }
    }
}

/// Static description of a table and its writable columns.
pub struct Resource {
    pub table: &'static str,
    pub columns: &'static [(&'static str, SqlType)],
}

impl Resource {
    #[must_use]
    pub fn column(&self, name: &str) -> Option<(&'static str, SqlType)> {
        self.columns
            .iter()
            .find(|(column, _)| *column == name)
            .copied()
    }

    #[must_use]
    pub fn has_column(&self, name: &str) -> bool {
        self.column(name).is_some()
    }
}

/// Everything a collection endpoint needs to expose filters, search and sort.
pub struct ListSpec {
    pub resource: &'static Resource,
    /// Columns accepted as `?column=value` exact-match filters.
    pub filterable: &'static [&'static str],
    /// Columns searched by `?q=`.
    pub searchable: &'static [&'static str],
    /// Columns accepted by `?sort=`.
    pub sortable: &'static [&'static str],
    /// Sort column used when `?sort=` is absent.
    pub default_sort: &'static str,
}

/// Converts a request DTO into a plain object for the query builders.
pub fn to_map<T: Serialize>(value: &T) -> ApiResult<Map<String, Value>> {
    match serde_json::to_value(value)? {
        Value::Object(map) => Ok(map),
        _ => Err(AppError::Internal(anyhow::anyhow!(
            "request payload did not serialize to an object"
        ))),
    }
}

/// Like [`to_map`], but assigns an application generated UUIDv7 primary key.
pub fn prepare_create<T: Serialize>(value: &T) -> ApiResult<Map<String, Value>> {
    let mut map = to_map(value)?;
    map.insert("id".to_string(), Value::String(Uuid::now_v7().to_string()));
    Ok(map)
}

fn placeholder(index: usize, ty: SqlType) -> String {
    match ty {
        SqlType::Text => format!("${index}"),
        other => format!("${index}::text::{}", other.cast()),
    }
}

fn bind_value(value: &Value, ty: SqlType) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(raw) if ty != SqlType::Jsonb => Some(raw.clone()),
        other => Some(other.to_string()),
    }
}

fn writable(resource: &Resource, key: &str) -> ApiResult<(&'static str, SqlType)> {
    resource.column(key).ok_or_else(|| {
        AppError::BadRequest(format!(
            "unknown or read-only field `{key}` for {}",
            resource.table
        ))
    })
}

fn build_insert(
    resource: &Resource,
    payload: &Map<String, Value>,
) -> ApiResult<(String, Vec<Option<String>>)> {
    let mut columns: Vec<(&'static str, SqlType)> = Vec::new();
    let mut values = Vec::new();

    for (key, value) in payload {
        if value.is_null() {
            continue;
        }
        let (name, ty) = writable(resource, key)?;
        columns.push((name, ty));
        values.push(bind_value(value, ty));
    }

    if columns.is_empty() {
        return Err(AppError::BadRequest(
            "request body must contain at least one field".into(),
        ));
    }

    let names = columns
        .iter()
        .map(|(name, _)| *name)
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = columns
        .iter()
        .enumerate()
        .map(|(index, (_, ty))| placeholder(index + 1, *ty))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "INSERT INTO {} ({names}) VALUES ({placeholders}) RETURNING *",
        resource.table,
    );

    Ok((sql, values))
}

fn build_update(
    resource: &Resource,
    payload: &Map<String, Value>,
) -> ApiResult<(String, Vec<Option<String>>)> {
    let mut assignments = Vec::new();
    let mut values = Vec::new();

    for (key, value) in payload {
        if value.is_null() || key == "id" {
            continue;
        }
        let (name, ty) = writable(resource, key)?;
        values.push(bind_value(value, ty));
        assignments.push(format!("{name} = {}", placeholder(values.len(), ty)));
    }

    if assignments.is_empty() {
        return Err(AppError::BadRequest("no updatable fields provided".into()));
    }

    if resource.has_column("updated_at") && !payload.contains_key("updated_at") {
        assignments.push("updated_at = now()".into());
    }

    let sql = format!(
        "UPDATE {} SET {} WHERE id = ${} RETURNING *",
        resource.table,
        assignments.join(", "),
        values.len() + 1
    );

    Ok((sql, values))
}

fn build_filters(spec: &ListSpec, params: &ListParams) -> ApiResult<(String, Vec<String>)> {
    let mut clauses = Vec::new();
    let mut values: Vec<String> = Vec::new();

    for (key, raw) in &params.filters {
        if !spec.filterable.contains(&key.as_str()) {
            return Err(AppError::BadRequest(format!(
                "unknown filter `{key}`; supported: {}",
                spec.filterable.join(", ")
            )));
        }
        let (name, ty) = writable(spec.resource, key)?;
        values.push(raw.clone());
        clauses.push(format!("{name} = {}", placeholder(values.len(), ty)));
    }

    if let Some(term) = params.q.as_deref().map(str::trim).filter(|q| !q.is_empty())
        && !spec.searchable.is_empty()
    {
        values.push(format!("%{term}%"));
        let index = values.len();
        let alternatives = spec
            .searchable
            .iter()
            .map(|column| format!("{column} ILIKE ${index}"))
            .collect::<Vec<_>>()
            .join(" OR ");
        clauses.push(format!("({alternatives})"));
    }

    let clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };

    Ok((clause, values))
}

fn build_order(spec: &ListSpec, params: &ListParams) -> ApiResult<String> {
    let (column, explicit) = match params.sort.as_deref() {
        Some(column) => (column, true),
        None => (spec.default_sort, false),
    };

    if !spec.sortable.contains(&column) {
        return Err(AppError::BadRequest(format!(
            "cannot sort by `{column}`; supported: {}",
            spec.sortable.join(", ")
        )));
    }

    let direction = match params.order.as_deref() {
        None if explicit => "ASC",
        None => "DESC",
        Some(order) if order.eq_ignore_ascii_case("asc") => "ASC",
        Some(order) if order.eq_ignore_ascii_case("desc") => "DESC",
        Some(order) => {
            return Err(AppError::BadRequest(format!(
                "invalid order `{order}`; use `asc` or `desc`"
            )));
        }
    };

    Ok(format!("ORDER BY {column} {direction}"))
}

/// Rows plus pagination metadata, before decoding into a response type.
pub struct RawPage {
    pub rows: Vec<PgRow>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl RawPage {
    pub fn into_page<T>(self) -> ApiResult<Page<T>>
    where
        T: for<'r> FromRow<'r, PgRow>,
    {
        let items = self
            .rows
            .iter()
            .map(T::from_row)
            .collect::<Result<Vec<_>, sqlx::Error>>()?;
        Ok(Page::new(items, self.total, self.limit, self.offset))
    }
}

pub async fn list(pool: &PgPool, spec: &ListSpec, params: &ListParams) -> ApiResult<RawPage> {
    let (filters, values) = build_filters(spec, params)?;
    let order = build_order(spec, params)?;
    let limit = params.limit();
    let offset = params.offset();

    let items_sql = format!(
        "SELECT * FROM {}{} {} LIMIT ${} OFFSET ${}",
        spec.resource.table,
        filters,
        order,
        values.len() + 1,
        values.len() + 2
    );
    let count_sql = format!("SELECT COUNT(*) FROM {}{}", spec.resource.table, filters);

    let mut items_query = sqlx::query(sqlx::AssertSqlSafe(items_sql));
    for value in &values {
        items_query = items_query.bind(value.clone());
    }
    let rows = items_query.bind(limit).bind(offset).fetch_all(pool).await?;

    let mut count_query = sqlx::query_scalar(sqlx::AssertSqlSafe(count_sql));
    for value in &values {
        count_query = count_query.bind(value.clone());
    }
    let total: i64 = count_query.fetch_one(pool).await?;

    Ok(RawPage {
        rows,
        total,
        limit,
        offset,
    })
}

pub async fn list_as<T>(pool: &PgPool, spec: &ListSpec, params: &ListParams) -> ApiResult<Page<T>>
where
    T: for<'r> FromRow<'r, PgRow>,
{
    list(pool, spec, params).await?.into_page()
}

pub async fn get(pool: &PgPool, resource: &Resource, id: Uuid) -> ApiResult<Option<PgRow>> {
    let sql = format!("SELECT * FROM {} WHERE id = $1", resource.table);
    Ok(sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_as<T>(pool: &PgPool, resource: &Resource, id: Uuid) -> ApiResult<Option<T>>
where
    T: for<'r> FromRow<'r, PgRow>,
{
    match get(pool, resource, id).await? {
        Some(row) => Ok(Some(T::from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn insert<'e, E>(
    executor: E,
    resource: &Resource,
    payload: &Map<String, Value>,
) -> ApiResult<PgRow>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let (sql, values) = build_insert(resource, payload)?;
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for value in values {
        query = query.bind(value);
    }
    Ok(query.fetch_one(executor).await?)
}

pub async fn insert_as<'e, E, T>(
    executor: E,
    resource: &Resource,
    payload: &Map<String, Value>,
) -> ApiResult<T>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    T: for<'r> FromRow<'r, PgRow>,
{
    let row = insert(executor, resource, payload).await?;
    Ok(T::from_row(&row)?)
}

pub async fn update(
    pool: &PgPool,
    resource: &Resource,
    id: Uuid,
    payload: &Map<String, Value>,
) -> ApiResult<Option<PgRow>> {
    let (sql, values) = build_update(resource, payload)?;
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql));
    for value in values {
        query = query.bind(value);
    }
    Ok(query.bind(id).fetch_optional(pool).await?)
}

pub async fn update_as<T>(
    pool: &PgPool,
    resource: &Resource,
    id: Uuid,
    payload: &Map<String, Value>,
) -> ApiResult<Option<T>>
where
    T: for<'r> FromRow<'r, PgRow>,
{
    match update(pool, resource, id, payload).await? {
        Some(row) => Ok(Some(T::from_row(&row)?)),
        None => Ok(None),
    }
}

pub async fn delete(pool: &PgPool, resource: &Resource, id: Uuid) -> ApiResult<bool> {
    let sql = format!("DELETE FROM {} WHERE id = $1", resource.table);
    Ok(sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}
