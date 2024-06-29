#[macro_use]
extern crate rocket;

use rocket::serde::json::{json, Value};
use rocket::State;
use rusql_alchemy::prelude::{config::db::Database, *};
use serde::Serialize;

#[derive(Clone)]
struct AppState {
    conn: Connection,
}

#[derive(Model, FromRow, Clone, Serialize)]
struct User_ {
    #[model(primary_key = true)]
    id: Serial,
    #[model(unique = true, null = false, size = 50)]
    username: String,
}

#[get("/users")]
async fn list_user(app_state: &State<AppState>) -> Value {
    let conn = app_state.conn.clone();
    let users = User_::all(&conn).await;
    
    ///  User_::get(kwargs!(Q!(user_id__eq : 5) & Q!(password__eq: "strongpassword")));
   
    json!(users)
}

#[main]
async fn main() {
    let conn = Database::new().await.conn;
    migrate!([User_], &conn);
    rocket::build()
        .mount("/", routes![list_user])
        .manage(AppState { conn })
        .launch()
        .await
        .expect("failed to launch rocket instance");
}
