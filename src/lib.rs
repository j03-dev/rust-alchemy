#[macro_export]
macro_rules! kwargs {
    ($($key:ident = $value:expr),*) => {
        {
            let mut args = Vec::new();
            $(
                args.push(rusql_alchemy::db::models::Arg {
                    key: stringify!($key).to_string(),
                    value: rusql_alchemy::to_value($value.clone()),
                    r#type: rusql_alchemy::get_type_name($value.clone()).into()
                });
            )*
            rusql_alchemy::db::models::Kwargs {
                operator: rusql_alchemy::db::models::Operator::And,
                args,
            }
        }
    };
}
use std::any::type_name;

pub fn get_type_name<T: Sized>(_: T) -> &'static str {
    type_name::<T>()
}

#[cfg(feature = "postgres")]
pub const PLACEHOLDER: &str = "$";

#[cfg(feature = "mysql")]
pub const PLACEHOLDER: &str = "?";

#[cfg(feature = "sqlite")]
pub const PLACEHOLDER: &str = "?";

pub fn to_value(value: impl Into<serde_json::Value>) -> serde_json::Value {
    let json_value = value.into();
    match json_value {
        serde_json::Value::Bool(true) => serde_json::json!(1),
        serde_json::Value::Bool(false) => serde_json::json!(0),
        _ => json_value,
    }
}

#[macro_export]
macro_rules! migrate {
    ([$($struct:ident),*], $conn:expr) => {
        $( $struct::migrate($conn).await; )*
    };
}

pub type Connection = sqlx::Pool<sqlx::Any>;

pub mod config {
    pub mod db {
        use sqlx::any::{install_default_drivers, AnyPoolOptions};

        use crate::Connection;
        async fn establish_connection(url: String) -> Connection {
            install_default_drivers();
            AnyPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .unwrap()
        }

        pub struct Database {
            pub conn: Connection,
        }

        impl Database {
            pub async fn new() -> Self {
                dotenv::dotenv().ok();
                let turso_database_url = std::env::var("DATABASE_URL").unwrap();

                Self {
                    conn: establish_connection(turso_database_url).await,
                }
            }
        }
    }
}

pub mod db {
    pub mod models {
        use crate::{get_type_name, Connection, PLACEHOLDER};

        use async_trait::async_trait;
        use serde_json::Value;
        use sqlx::{any::AnyRow, FromRow, Row};

        pub type Serial = i32;
        pub type Integer = i32;
        pub type Text = String;
        pub type Float = f64;
        pub type Date = String;
        pub type DateTime = String;
        pub type Boolean = i32;

        #[derive(Debug)]
        pub enum Operator {
            Or,
            And,
        }

        impl Operator {
            fn get(&self) -> &'static str {
                match self {
                    Self::Or => " or ",
                    Self::And => " and ",
                }
            }
        }

        #[derive(Debug)]
        pub struct Arg {
            pub key: String,
            pub value: Value,
            pub r#type: String,
        }

        #[derive(Debug)]
        pub struct Kwargs {
            pub operator: Operator,
            pub args: Vec<Arg>,
        }

        impl Kwargs {
            pub fn or(self) -> Self {
                Self {
                    operator: Operator::Or,
                    args: self.args,
                }
            }
        }

        #[async_trait]
        pub trait Model<R: Row>: Clone + Sync + for<'r> FromRow<'r, R> {
            const SCHEMA: &'static str;
            const NAME: &'static str;
            const PK: &'static str;

            async fn migrate(conn: &Connection) -> bool
            where
                Self: Sized,
            {
                match sqlx::query(Self::SCHEMA).execute(conn).await {
                    Ok(_) => true,
                    Err(err) => {
                        eprintln!("{err}");
                        false
                    }
                }
            }

            async fn update(&self, conn: &Connection) -> bool
            where
                Self: Sized;

            async fn set<T: ToString + Clone + Send + Sync>(
                id_value: T,
                kw: Kwargs,
                conn: &Connection,
            ) -> bool {
                let mut fields = Vec::new();
                let mut values = Vec::new();

                for (i, arg) in kw.args.iter().enumerate() {
                    fields.push(format!("{}={PLACEHOLDER}{}", arg.key, i + 1,));
                    values.push((arg.r#type.clone(), arg.value.to_string()));
                }
                values.push((
                    get_type_name(id_value.clone()).to_string(),
                    id_value.clone().to_string(),
                ));
                let j = fields.len() + 1;
                let fields = fields.join(", ");
                let query = format!(
                    "update {name} set {fields} where {id}={PLACEHOLDER}{j};",
                    id = Self::PK,
                    name = Self::NAME,
                );
                let mut stream = sqlx::query(&query);
                for (t, v) in values {
                    match t.as_str() {
                        "i32" => {
                            stream = stream.bind(v.replace('"', "").parse::<i32>().unwrap());
                        }
                        "f64" => {
                            stream = stream.bind(v.replace('"', "").parse::<f64>().unwrap());
                        }
                        _ => {
                            stream = stream.bind(v.replace('"', ""));
                        }
                    }
                }
                println!("{}", query);
                if let Err(err) = stream.execute(conn).await {
                    println!("{}", err);
                    false
                } else {
                    true
                }
            }

            async fn save(&self, conn: &Connection) -> bool
            where
                Self: Sized;

            async fn create(kw: Kwargs, conn: &Connection) -> bool
            where
                Self: Sized,
            {
                let mut fields = Vec::new();
                let mut values = Vec::new();
                let mut placeholder = Vec::new();

                for (i, arg) in kw.args.iter().enumerate() {
                    fields.push(arg.key.to_owned());
                    values.push((arg.r#type.clone(), arg.value.to_string()));
                    placeholder.push(format!("{PLACEHOLDER}{}", i + 1));
                }

                let fields = fields.join(", ");
                let placeholder = placeholder.join(", ");
                let query = format!(
                    "insert into {name} ({fields}) values ({placeholder});",
                    name = Self::NAME
                );
                let mut stream = sqlx::query(&query);
                for (t, v) in values {
                    match t.as_str() {
                        "i32" => {
                            stream = stream.bind(v.replace('"', "").parse::<i32>().unwrap());
                        }
                        "f64" => {
                            stream = stream.bind(v.replace('"', "").parse::<f64>().unwrap());
                        }
                        _ => {
                            stream = stream.bind(v.replace('"', ""));
                        }
                    }
                }
                stream.execute(conn).await.is_ok()
            }

            async fn all(conn: &Connection) -> Vec<Self>
            where
                Self: Sized + std::marker::Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
            {
                let query = format!("select * from {name}", name = Self::NAME);
                match sqlx::query_as::<_, Self>(&query).fetch_all(conn).await {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("{}", err);
                        Vec::new()
                    }
                }
            }

            async fn filter(kw: Kwargs, conn: &Connection) -> Vec<Self>
            where
                Self: Sized + std::marker::Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
            {
                let mut fields = Vec::new();
                let mut values = Vec::new();

                let mut join_query = None;

                for (i, arg) in kw.args.iter().enumerate() {
                    let parts: Vec<&str> = arg.key.split("__").collect();
                    values.push((arg.r#type.clone(), arg.value.to_string()));
                    match parts.as_slice() {
                        [field_a, table, field_b] if parts.len() == 3 => {
                            join_query = Some(format!(
                                "INNER JOIN {table} ON {name}.{pk} = {table}.{field_a}",
                                name = Self::NAME,
                                pk = Self::PK
                            ));
                            fields.push(format!("{table}.{field_b}={PLACEHOLDER}{}", i + 1));
                        }
                        _ => fields.push(format!("{}={PLACEHOLDER}{}", arg.key, i + 1)),
                    }
                }
                let fields = fields.join(kw.operator.get());
                let query = if let Some(join) = join_query {
                    format!(
                        "SELECT {name}.* FROM {name} {join} WHERE {fields};",
                        name = Self::NAME
                    )
                } else {
                    format!("SELECT * FROM {name} WHERE {fields};", name = Self::NAME)
                };

                let stream = sqlx::query_as::<_, Self>(&query);
                let mut stream = stream;
                for (t, v) in values {
                    match t.as_str() {
                        "i32" => {
                            stream = stream.bind(v.replace('"', "").parse::<i32>().unwrap());
                        }
                        "f64" => {
                            stream = stream.bind(v.replace('"', "").parse::<f64>().unwrap());
                        }
                        _ => {
                            stream = stream.bind(v.replace('"', ""));
                        }
                    }
                }
                if let Ok(result) = stream.fetch_all(conn).await {
                    return result;
                } else {
                    return Vec::new();
                }
            }

            async fn get(kw: Kwargs, conn: &Connection) -> Option<Self>
            where
                Self: Sized + std::marker::Unpin + for<'r> FromRow<'r, AnyRow> + Clone,
            {
                let result = Self::filter(kw, conn).await;
                if let Some(r) = result.first() {
                    return Some(r.to_owned());
                }
                None
            }

            async fn delete(&self, conn: &Connection) -> bool
            where
                Self: Sized;

            async fn count(&self, conn: &Connection) -> usize
            where
                Self: Sized,
            {
                let query = format!("select count(*) from {name}", name = Self::NAME);
                sqlx::query(query.as_str())
                    .fetch_one(conn)
                    .await
                    .map_or(0, |r| r.get::<i64, _>(0) as usize)
            }
        }

        #[async_trait]
        pub trait Delete {
            async fn delete(&self, conn: &Connection) -> bool;
        }

        #[async_trait]
        impl<T> Delete for Vec<T>
        where
            T: Model<AnyRow>
                + Clone
                + Sync
                + Send
                + std::marker::Unpin
                + for<'r> FromRow<'r, AnyRow>,
        {
            async fn delete(&self, conn: &Connection) -> bool {
                let query = format!("delete from {name}", name = T::NAME);
                sqlx::query(query.as_str()).execute(conn).await.is_ok()
            }
        }
    }
}

pub mod prelude {
    pub use crate::Connection;
    pub use crate::{
        config,
        db::models::{Boolean, Date, DateTime, Delete, Float, Integer, Model, Serial, Text},
        kwargs, migrate,
    };
    pub use async_trait::async_trait;
    pub use rusql_alchemy_macro::Model;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json;
}
