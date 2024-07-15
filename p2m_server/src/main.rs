#[macro_use] extern crate rocket;

use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    content: String,
}

#[post("/message", format = "json", data = "<message>")]
fn post_message(message: Json<Message>) -> Json<Message> {
    Json(Message {
        content: format!("Received: {}", message.content),
    })
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index])
        .mount("/api", routes![post_message])
}
