#![allow(clippy::wrong_self_convention)]

use crate::cores::database::connection::ConnectionPool;
use crate::cores::database::entity::Entity;
use crate::cores::database::expr::and_x::AndX;
use crate::cores::database::expr::expr::{Expr, Expression};
use crate::cores::database::expr::join::{Join, JoinType};
use crate::cores::database::expr::or_x::OrX;
use crate::cores::helper::hack::Hack;
use sqlx::{Database, Encode, Error, Postgres, Type};
use std::any::type_name;
use std::fmt::{Debug};
use std::marker::PhantomData;

#[derive(Debug, Clone, PartialEq)]
enum QueryKind {
    Select {
        select: Vec<String>,
        table: Option<String>,
        alias: Option<String>,
        table_schema: Option<String>,
    },
    Update {
        table: Option<String>,
        alias: Option<String>,
        table_schema: Option<String>,
        sets: Vec<(String, String)>,
    },
    Delete {
        table: Option<String>,
        alias: Option<String>,
        table_schema: Option<String>,
    },
}

impl QueryKind {
    fn get_alias(&self) -> Option<String> {
        match self {
            QueryKind::Select { alias, .. }
            | QueryKind::Update { alias, .. }
            | QueryKind::Delete { alias, .. } => alias.clone().map(|e| e.clone()),
        }
    }
    fn get_table(&self) -> Option<String> {
        match self {
            QueryKind::Select { table, .. }
            | QueryKind::Update { table, .. }
            | QueryKind::Delete { table, .. } => table.clone().map(|e| e.clone()),
        }
    }
    fn get_table_schema(&self) -> Option<String> {
        match self {
            QueryKind::Select { table_schema, .. }
            | QueryKind::Update { table_schema, .. }
            | QueryKind::Delete { table_schema, .. } => table_schema.clone().map(|e| e.clone()),
        }
    }
    fn get_selects(&self) -> Option<Vec<String>> {
        match self {
            QueryKind::Select { select, .. } => Some(select.clone()),
            _ => None,
        }
    }
    fn is_select(&self) -> bool {
        matches!(self, QueryKind::Select { .. })
    }
    fn is_update(&self) -> bool {
        matches!(self, QueryKind::Update { .. })
    }
    fn is_delete(&self) -> bool {
        matches!(self, QueryKind::Delete { .. })
    }
    fn add_select(&mut self, field: String) {
        if let QueryKind::Select { select, .. } = self {
            select.push(field);
        }
    }
    fn set_select(&mut self, fields: Vec<String>) {
        if let QueryKind::Select { select, .. } = self {
            *select = fields;
        }
    }
    fn set_schema(&mut self, schema: Option<String>) {
        match self {
            QueryKind::Select { table_schema, .. }
            | QueryKind::Update { table_schema, .. }
            | QueryKind::Delete { table_schema, .. } => {
                *table_schema = schema;
            }
        }
    }
    fn set_alias(&mut self, alias: Option<String>) {
        match self {
            QueryKind::Select { alias: a, .. }
            | QueryKind::Update { alias: a, .. }
            | QueryKind::Delete { alias: a, .. } => {
                *a = alias;
            }
        }
    }
    fn set_table(&mut self, table: String) {
        match self {
            QueryKind::Select { table: t, .. }
            | QueryKind::Update { table: t, .. }
            | QueryKind::Delete { table: t, .. } => {
                *t = Some(table);
            }
        }
    }
    fn from(&mut self, table: String, alias: Option<String>, schema: Option<String>) {
        match self {
            QueryKind::Select {
                table: t,
                alias: a,
                table_schema: s,
                ..
            }
            | QueryKind::Update {
                table: t,
                alias: a,
                table_schema: s,
                ..
            }
            | QueryKind::Delete {
                table: t,
                alias: a,
                table_schema: s,
                ..
            } => {
                *t = Some(table);
                *a = alias;
                *s = schema;
            }
        }
    }

    fn to_sql(&self) -> Result<String, Error> {
        let mut sql = String::new();
        match self {
            QueryKind::Select { select, .. } => {
                sql.push_str("SELECT");
                sql.push(' ');
                if select.is_empty() {
                    sql.push_str("*");
                } else {
                    sql.push_str(
                        &select
                            .iter()
                            .map(Hack::escape_table_identifier_quote)
                            .collect::<Vec<String>>()
                            .join(", "),
                    );
                }
            }
            QueryKind::Update { .. } => sql.push_str("UPDATE"),
            QueryKind::Delete { .. } => sql.push_str("DELETE FROM"),
        }
        sql.push(' ');
        let table = self.get_table().ok_or(Error::Protocol("Table name is empty".to_string()))?;
        let table_schema = self.get_table_schema();
        if let Some(schema) = table_schema {
            sql.push_str(&format!(
                "{}.{}",
                Hack::escape_table_identifier_quote(schema),
                Hack::escape_table_identifier_quote(table)
            ));
        } else {
            sql.push_str(&Hack::escape_table_identifier_quote(table));
        }
        if let Some(alias) = self.get_alias() {
            sql.push(' ');
            sql.push_str(&Hack::escape_table_identifier_quote(alias));
        }
        if matches!(self, QueryKind::Update { .. }) {
            if let QueryKind::Update { sets, .. } = self {
                if sets.is_empty() {
                    return Err(Error::Protocol(
                        "At least one column must be set for update query".to_string(),
                    ));
                }
                sql.push_str(" SET ");
                // set with get indexed column name and value $1, $2
                let sets = sets
                    .iter()
                    .enumerate()
                    .map(|(i, (col, _))| {
                        format!("{} = ${}", Hack::escape_table_identifier_quote(col), i + 1)
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                sql.push_str(&sets);
            }
        }
        Ok(sql)
    }
}

#[derive(Debug, Clone)]
pub struct QueryBuilder<'a, E: Entity> {
    pool: &'a ConnectionPool,
    _marker: PhantomData<E>,
    kind: QueryKind,
    joins: Vec<Join>,

    where_clause: Option<Box<dyn Expression>>,

    group_by: Vec<String>,
    having: Option<Box<dyn Expression>>,

    order_by: Vec<String>,
    limit: Option<u64>,
}

impl<'a, E: Entity> QueryBuilder<'a, E>
where
    for<'q> <Postgres as Database>::Arguments<'q>: sqlx::IntoArguments<'q, Postgres>,
    for<'c> &'c ConnectionPool: sqlx::Executor<'c, Database = Postgres>,
{
    pub fn new(pool: &'a ConnectionPool) -> Self {
        Self {
            pool,
            where_clause: None,
            order_by: Vec::new(),
            limit: None,
            having: None,
            joins: vec![],
            group_by: vec![],
            _marker: PhantomData,
            kind: QueryKind::Select {
                select: Vec::new(),
                alias: None,
                table: None,
                table_schema: None,
            },
        }
    }

    fn get_pool(&self) -> &'a ConnectionPool {
        self.pool
    }

    /* =========================
     * SELECT
     * ========================= */
    pub fn select<I, S>(&mut self, fields: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if self.kind.is_select() {
            self.kind
                .set_select(fields.into_iter().map(|s| s.into()).collect());
        } else {
            self.kind = QueryKind::Select {
                select: fields.into_iter().map(|s| s.into()).collect(),
                alias: None,
                table: None,
                table_schema: None,
            };
        }
        self
    }

    pub fn add_select<S: Into<String>>(&mut self, field: S) -> &mut Self {
        self.kind.add_select(field.into());
        self
    }

    /* =========================
     * FROM
     * ========================= */

    pub fn from<T: Into<String>, A: Into<String>, S: Into<String>>(
        &mut self,
        table: T,
        alias: Option<A>,
        schema: Option<S>,
    ) -> &mut Self {
        self.kind.from(
            table.into(),
            alias.map(|e| e.into()),
            schema.map(|e| e.into()),
        );
        self
    }

    pub fn from_table<T: Into<String>>(&mut self, table: T) -> &mut Self {
        self.kind.set_table(table.into());
        self
    }

    pub fn from_schema<T: Into<String>, A: Into<String>>(
        &mut self,
        schema: Option<T>,
    ) -> &mut Self {
        self.kind.set_schema(schema.map(|e| e.into()));
        self
    }

    /* =========================
     * WHERE
     * ========================= */
    pub fn where_(&mut self, expr: impl Expression + 'static) -> &mut Self {
        self.where_clause = Some(Box::new(expr));
        self
    }

    pub fn and_where(&mut self, expr: impl Expression + 'static) -> &mut Self {
        self.where_clause = Some(match &self.where_clause {
            Some(prev) => Box::new(AndX::new(vec![prev.clone(), Box::new(expr)])),
            None => Box::new(expr),
        });
        self
    }

    pub fn or_where(&mut self, expr: impl Expression + 'static) -> &mut Self {
        self.where_clause = Some(match &self.where_clause {
            Some(prev) => Box::new(OrX::new(vec![prev.clone(), Box::new(expr)])),
            None => Box::new(expr),
        });
        self
    }

    /* =========================
     * ORDER BY
     * ========================= */
    pub fn order_by<S: Into<String>>(&mut self, field: S) -> &mut Self {
        self.order_by = vec![field.into()];
        self
    }

    pub fn add_order_by<S: Into<String>>(&mut self, field: S) -> &mut Self {
        self.order_by.push(field.into());
        self
    }

    /* =========================
     * LIMIT
     * ========================= */
    pub fn join<J: Into<String>>(&mut self, join: J) -> &mut Self {
        self.joins.push(Join::new(JoinType::Inner, join));
        self
    }

    pub fn left_join<J: Into<String>>(&mut self, join: J) -> &mut Self {
        self.joins.push(Join::new(JoinType::Left, join));
        self
    }

    /* =========================
     * LIMIT
     * ========================= */
    pub fn limit(&mut self, n: u64) -> &mut Self {
        self.limit = Some(n);
        self
    }
    /* =========================
     * GROUP BY
     * ========================= */
    pub fn group_by<S: Into<String>>(&mut self, field: S) -> &mut Self {
        self.group_by.push(field.into());
        self
    }

    pub fn add_group_by<S: Into<String>>(&mut self, field: S) -> &mut Self {
        self.group_by.push(field.into());
        self
    }
    /* =========================
     * HAVING
     * ========================= */
    pub fn having(&mut self, expr: impl Expression + 'static) -> &mut Self {
        self.having = Some(Box::new(expr));
        self
    }

    pub fn and_having(&mut self, expr: impl Expression + 'static) -> &mut Self {
        self.having = Some(match &self.having {
            Some(prev) => Box::new(AndX::new(vec![prev.clone(), Box::new(expr)])),
            None => Box::new(expr),
        });
        self
    }

    pub fn or_having(&mut self, expr: impl Expression + 'static) -> &mut Self {
        self.having = Some(match &self.having {
            Some(prev) => Box::new(OrX::new(vec![prev.clone(), Box::new(expr)])),
            None => Box::new(expr),
        });
        self
    }

    /* =========================
     * SQL STRING
     * ========================= */
    pub fn to_sql(&self) -> Result<String, Error> {
        let mut sql = self.kind.to_sql()?;

        /* =========================
         * JOIN / LEFT JOIN
         * ========================= */
        for join in &self.joins {
            sql.push(' ');
            sql.push_str(&join.to_string());
        }

        /* =========================
         * WHERE
         * ========================= */
        if let Some(w) = &self.where_clause {
            sql.push_str(" WHERE ");
            sql.push_str(&w.to_string());
        }

        /* =========================
         * GROUP BY
         * ========================= */
        if !self.group_by.is_empty() {
            sql.push_str(" GROUP BY ");
            sql.push_str(&self.group_by.join(", "));
        }

        /* =========================
         * HAVING
         * ========================= */
        if let Some(h) = &self.having {
            sql.push_str(" HAVING ");
            sql.push_str(&h.to_string());
        }

        /* =========================
         * ORDER BY
         * ========================= */
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            sql.push_str(&self.order_by.join(", "));
        }

        /* =========================
         * LIMIT
         * ========================= */
        if let Some(l) = self.limit {
            sql.push_str(" LIMIT ");
            sql.push_str(&l.to_string());
        }

        Ok(sql)
    }

    pub fn update_entity<Ent: Entity>(&mut self, schema: Option<String>) -> &mut Self {
        self.kind = QueryKind::Update {
            table: Some(Ent::table()),
            sets: Vec::new(),
            alias: None,
            table_schema: schema,
        };
        self
    }

    pub fn set<C, V>(&mut self, column: C, value: V) -> Result<&mut Self, Error>
    where
        C: Into<String>,
        V: Into<String>,
    {
        match &mut self.kind {
            QueryKind::Update { sets, .. } => {
                sets.push((column.into(), value.into()));
                Ok(self)
            }
            _ => Err(Error::Protocol(
                "QueryBuilder.set() only works with update query".to_string(),
            )),
        }
    }

    pub fn set_col<V, C>(&mut self, col: C, value: V) -> Result<&mut Self, Error>
    where
        V: Into<String>,
        C: Into<String>,
    {
        match &mut self.kind {
            QueryKind::Update { sets, .. } => {
                sets.push((col.into(), value.into()));
                Ok(self)
            }
            _ => Err(Error::Protocol(
                "QueryBuilder.set_col() only works with update query".to_string(),
            )),
        }
    }

    pub fn delete_entity<Ent: Entity>(&mut self, schema: Option<String>) -> &mut Self {
        self.kind = QueryKind::Delete {
            table: Some(E::table()),
            alias: None,
            table_schema: schema,
        };
        self
    }

    pub fn delete(&mut self) -> &mut Self {
        if self.kind.is_delete() {
            return self;
        }
        self.kind = QueryKind::Delete {
            table: self.kind.get_table(),
            alias: self.kind.get_alias(),
            table_schema: self.kind.get_table_schema(),
        };
        self
    }
    pub fn clone_into_select(&self) -> Self {
        let mut new = self.clone();
        new.kind = QueryKind::Select {
            select: Vec::new(),
            alias: self.kind.get_alias(),
            table: self.kind.get_table(),
            table_schema: self.kind.get_table_schema(),
        };
        new
    }
    pub fn clone_into_update(&self) -> Self {
        let mut new = self.clone();
        new.kind = QueryKind::Update {
            table: self.kind.get_table(),
            sets: Vec::new(),
            alias: self.kind.get_alias(),
            table_schema: self.kind.get_table_schema(),
        };
        new
    }
    pub fn clone_into_delete(&self) -> Self {
        let mut new = self.clone();
        new.kind = QueryKind::Delete {
            table: self.kind.get_table(),
            alias: self.kind.get_alias(),
            table_schema: self.kind.get_table_schema(),
        };
        new
    }
    pub async fn find_as<Ent: Entity>(
        pool: &'a ConnectionPool,
        id: Ent::KeyType,
    ) -> Result<Ent, Error>
    where
        Ent::KeyType: for<'q> Encode<'q, Postgres> + Type<Postgres>,
    {
        let primary_key = Ent::primary_key();
        if primary_key.is_empty() {
            return Err(Error::InvalidArgument(format!(
                "Entity {} does not support primary key",
                type_name::<Ent>()
            )));
        }
        let expr = Expr::eq(primary_key, id.to_string());
        Ok(QueryBuilder::<'a, Ent>::new(pool)
            .select(vec!['*'])
            .where_(expr)
            .fetch_one_as()
            .await?)
    }

    pub async fn find_where_as<Ent, V, C>(
        pool: &'a ConnectionPool,
        column: C,
        value: V,
    ) -> Result<E, Error>
    where
        Ent: Entity,
        C: Into<String>,
        V: for<'q> Encode<'q, Postgres>
            + Type<Postgres>
            + Send
            + Sync
            + Into<String>
            + Debug
            + 'static,
    {
        let expr = Expr::eq(column, value);
        Ok(QueryBuilder::<'a, E>::new(pool)
            .select(vec!['*'])
            .where_(expr)
            .fetch_one_as()
            .await?)
    }

    // /* =========================
    //  * EXECUTION
    //  * ========================= */
    //
    pub async fn fetch_all_as<Ent>(&self) -> Result<Vec<Ent>, Error>
    where
        Ent: Entity,
    {
        let sql = self.to_sql()?;
        sqlx::query_as::<Postgres, Ent>(&sql)
            .fetch_all(self.pool)
            .await
    }

    pub async fn fetch_one_as<Ent>(&self) -> Result<Ent, Error>
    where
        Ent: Entity,
    {
        let sql = self.to_sql()?;
        sqlx::query_as::<Postgres, Ent>(&sql)
            .fetch_one(self.get_pool())
            .await
    }

    pub async fn execute(&self) -> Result<<Postgres as Database>::QueryResult, Error> {
        let sql = self.to_sql()?;
        sqlx::query(&sql).execute(self.pool).await
    }
}
