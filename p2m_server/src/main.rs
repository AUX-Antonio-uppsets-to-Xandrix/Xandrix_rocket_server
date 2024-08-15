#[macro_use] extern crate rocket;

use rocket::fs::NamedFile;
use rocket::fs::{FileServer, relative};
use rocket::http::ContentType;
use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};
use rocket::Data;
use std::fs::File;
use std::io::{Write, Cursor};
use std::path::{Path, PathBuf};
use rocket_multipart_form_data::{MultipartFormDataOptions, MultipartFormData, MultipartFormDataField};
use image::io::Reader as ImageReader;
use image::{DynamicImage, GenericImageView};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    content: String,
}

#[post("/upload", data = "<data>")]
async fn upload(content_type: &ContentType, data: Data<'_>) -> Result<String, &'static str> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::raw("file").size_limit(5 * 1024 * 1024),
    ]);
    
    let multipart_form_data = match MultipartFormData::parse(content_type, data, options).await {
        Ok(form) => form,
        Err(_) => return Err("Error parsing form data"),
    };

    let raw = match multipart_form_data.raw.get("file") {
        Some(file) => file,
        None => return Err("File field is missing"),
    };

    let raw_field = match raw.get(0) {
        Some(field) => field,
        None => return Err("File field is empty"),
    };

    let image = match ImageReader::new(Cursor::new(&raw_field.raw)).with_guessed_format() {
        Ok(reader) => match reader.decode() {
            Ok(img) => img,
            Err(_) => return Err("Failed to decode image"),
        },
        Err(_) => return Err("Failed to read image format"),
    };

    // 예제: PNG 이미지를 OBJ 데이터로 변환 (단순한 예시)
    let (width, height) = image.dimensions();
    let obj_data = format!(
        "o Object\nv 0.0 0.0 0.0\nv {} 0.0 0.0\nv {} {} 0.0\nv 0.0 {} 0.0\nf 1 2 3 4\n",
        width, width, height, height
    );

    let obj_file_path = format!("./uploads/output.obj");
    let mut obj_file = File::create(&obj_file_path).map_err(|_| "Failed to create OBJ file")?;
    obj_file.write_all(obj_data.as_bytes()).map_err(|_| "Failed to write OBJ file")?;

    Ok(obj_file_path)

}
#[get("/download/<file..>")]
async fn download(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("uploads/").join(file)).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, upload, download])
}