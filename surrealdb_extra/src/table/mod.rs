//! The `Table` trait represents a database table and provides common operations for interacting with the table.
//!
//! To use this trait, implement it for your struct representing the table. The struct must have the following attributes:
//! - `#[derive(Table)]` to automatically derive the implementation of the `Table` trait.
//! - `#[table(name = "...")]` to specify the name of the table in the database.
//! - `id: Option<RecordId>` needs to be one of the fields
//!
//! # Example
//!
//!
//! ``` rust
//!  use serde::{Serialize, Deserialize};
//!  use surrealdb_extra::table::Table;
//!  use surrealdb::sql::Thing as RecordId;
//!  use surrealdb::engine::any::connect;
//!  use surrealdb::{Surreal, Result};
//!  use tokio::main;
//!  use surrealdb::kvs::Datastore;
//!
//! // Serialize and Deserialize are must have derives
//! #[derive(Table, Serialize, Deserialize, Clone)]
//! #[table(name = "my_table")]
//! struct MyStruct {
//!  // id is the only field that is a must id must be of type Option<::surrealdb::sql::Thing>
//!     id: Option<RecordId>,
//!     // other fields...
//!     pub name: String
//! }
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let db = connect("mem://").await.unwrap();
//!     db.use_ns("ns").use_db("db").await.unwrap();
//!
//!     let my_struct = MyStruct {
//!         id: None,
//!         name: "my name is".into()
//!     };
//!
//!     let created_struct = my_struct.create(&db).await.unwrap();
//!
//!     let mut updated_struct = created_struct.first().unwrap().clone();
//!     updated_struct.name = "test".to_string();
//!
//!     let updated_struct: Option<MyStruct> = updated_struct.update(&db).await.unwrap();
//!
//!     let deleted_struct: Option<MyStruct> = MyStruct::delete(&db, updated_struct.unwrap().id.unwrap().to_raw()).await.unwrap();
//!
//!     let get_all: Vec<MyStruct> = MyStruct::get_all(&db).await.unwrap();
//!
//!     let get_by_id: Option<MyStruct> = MyStruct::get_by_id(&db, "id").await.unwrap();
//!
//!     Ok(())
//! }
//! ```

pub mod err;

#[cfg(feature = "derive")]
pub use ::surrealdb_extra_derive::Table;

use anyhow::Result;
use ::async_trait::async_trait;
use ::serde::de::DeserializeOwned;
use ::serde::Serialize;
use ::surrealdb::{Connection, Surreal};
pub use crate::table::err::TableError;

#[cfg(feature = "query")]
use surrealdb::sql::Thing as RecordId;

#[cfg(feature = "query")]
use crate::query::{
    select::SelectBuilder,
    update::UpdateBuilder,
    create::CreateBuilder,
    statement::StatementBuilder,
    states::{FilledWhat, NoFields, NoCond, FilledData}
};

#[async_trait]
pub trait Table: Serialize + DeserializeOwned + Send + Sync where Self: 'static
{
    const TABLE_NAME: &'static str;

    fn get_id(&self) -> &Option<::surrealdb::sql::Thing>;

    fn set_id(&mut self, id: impl Into<::surrealdb::sql::Id>);

    fn create_record_id(id: impl Into<::surrealdb::sql::Id>) -> ::surrealdb::sql::Thing {
        ::surrealdb::sql::Thing::from((Self::TABLE_NAME, id.into()))
    }

    async fn create<C: Connection>(self, db: &Surreal<C>) -> Result<Vec<Self>> {
        let s: Vec<Self> = db.create(Self::TABLE_NAME).content(self).await?;

        Ok(s)
    }

    async fn delete<C: Connection>(db: &Surreal<C>, id: impl Into<String> + Send) -> Result<Option<Self>> {
        let s: Option<Self> = db.delete((Self::TABLE_NAME, id.into())).await?;

        Ok(s)
    }

    async fn get_all<C: Connection>(db: &Surreal<C>) -> Result<Vec<Self>> {
        let vec_s: Vec<Self> = db.select(Self::TABLE_NAME).await?;

        Ok(vec_s)
    }

    async fn get_by_id<C: Connection>(db: &Surreal<C>, id: impl Into<String> + Send) -> Result<Option<Self>> {
        let s: Option<Self> = db.select((Self::TABLE_NAME, id.into())).await?;

        Ok(s)
    }

    /// This function works best with 'serde_with::skip_serializing_none' reason is so that if the option value none does not override the database if filled
    /// Of course using 'serde_with::skip_serializing_none' is optional
    ///
    /// Example:
    /// ```rust
    /// use serde::{Deserialize, Serialize};
    /// use serde_with::skip_serializing_none;
    /// use surrealdb::sql::Thing as RecordId;
    /// use surrealdb_extra::table::Table;
    ///
    /// #[skip_serializing_none]
    /// #[derive(Debug, Table, Serialize, Deserialize)]
    /// #[table(name = "test")]
    /// struct Test {
    ///     id: Option<RecordId>,
    /// }
    /// ```
    async fn update<C: Connection>(self, db: &Surreal<C>) -> Result<Option<Self>> {
        let s: Option<Self> = db
            .update(
                (
                    Self::TABLE_NAME,
                    self.get_id().clone().ok_or(TableError::IdEmpty)?.id.to_owned().to_raw()
                )
            )
            .merge(self)
            .await?;

        Ok(s)
    }

    #[cfg(feature = "query")]
    fn select_builder<C: Connection>(db: &Surreal<C>, id: Option<String>) -> SelectBuilder<C, FilledWhat, NoFields, NoCond> {
        if let Some(id) = id {
            return db.select_builder().what(RecordId::from((Self::TABLE_NAME, id.as_str())))
        }

        db.select_builder().what(Self::TABLE_NAME)
    }

    // It auto fills the content if this is not what you want use the `UpdateBuilder`
    #[cfg(feature = "query")]
    fn update_builder<C: Connection>(self, db: &Surreal<C>) -> UpdateBuilder<C, FilledWhat, FilledData, NoCond> {
        db.update_builder().what(Self::TABLE_NAME).content(self)
    }


    // It auto fills the content if this is not what you want use the `CreateBuilder`
    #[cfg(feature = "query")]
    fn create_builder<C: Connection>(self, db: &Surreal<C>) -> CreateBuilder<C, FilledWhat, FilledData> {
        db.create_builder().what(Self::TABLE_NAME).content(self)
    }
}
