use rusql_alchemy::prelude::*;
use sqlx::FromRow;

#[derive(FromRow, Clone, Debug, Default, Model)]
struct User {
    #[model(primary_key = true, auto = true, null = false)]
    id: Integer,
    #[model(size = 50, unique = true, null = false)]
    name: String,
    #[model(size = 255, unique = true, null = true)]
    email: String,
    #[model(size = 255, null = false)]
    password: String,
    #[model(default = "user")]
    role: String,
}

#[derive(FromRow, Debug, Default, Model, Clone)]
struct Product {
    #[model(primary_key = true, auto = true, null = false)]
    id: Integer,
    #[model(size = 50, null = false)]
    name: String,
    price: Float,
    description: Text,
    #[model(default = "now")]
    at: DateTime,
    #[model(default = true)]
    is_sel: Boolean,
    #[model(null = false, foreign_key = "User.id")]
    owner: Integer,
}

#[tokio::main]
async fn main() {
    let conn = config::db::Database::new().await.conn;

    migrate!([User, Product], &conn);

    println!(
        "is save {}",
        User {
            name: "johnDoe@gmail.com".to_string(),
            email: "21john@gmail.com".to_string(),
            password: "p455w0rd".to_string(),
            ..Default::default()
        }
        .save(&conn)
        .await
    );

    let users = User::all(&conn).await;
    println!("{:#?}", users);

    User::create(
        kwargs!(
            name = "joe",
            email = "24nomeniavo@gmail.com",
            password = "strongpassword"
        ),
        &conn,
    )
    .await;

    let users = User::all(&conn).await;
    println!("1: {:#?}", users);

    if let Some(user) = User::get(
        kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
        &conn,
    )
    .await
    {
        User {
            role: "admin".into(),
            ..user
        }
        .update(&conn)
        .await;
    }
    let user = User::get(
        kwargs!(email = "24nomeniavo@gmail.com", password = "strongpassword"),
        &conn,
    )
    .await;

    println!("2: {:#?}", user);

    Product::create(
        kwargs!(
            name = "tomato".to_string(),
            price = 1000.0,
            description = "".to_string(),
            owner = user.clone().unwrap().id
        ),
        &conn,
    )
    .await;

    let products = Product::all(&conn).await;
    println!("3: {:#?}", products);

    let product = Product::get(kwargs!(is_sel = true), &conn).await;
    println!("4: {:#?}", product);

    let user = User::get(kwargs!(owner__product__is_sel = true), &conn).await;
    println!("5: {:#?}", user);

    println!("is deleted = {}", products.delete(&conn).await);

    let products = Product::all(&conn).await;
    println!("6: {:#?}", products);
}
