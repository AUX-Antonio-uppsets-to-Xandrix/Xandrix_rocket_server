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
use image::{DynamicImage, ImageOutputFormat, GenericImageView};
use std::process::Command;
use std::fs;
use rocket::tokio::time::{sleep, Duration};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    content: String,
}

fn save_image(image: &DynamicImage, path: &str) -> Result<(), std::io::Error> {
    let mut file = File::create(path)?;
    image.write_to(&mut file, ImageOutputFormat::Png).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Failed to save image"))?;
    Ok(())
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

    let img_file_path = format!("../../meshrnn/meshrcnn/input/input.png");
    if let Err(_) = save_image(&image, &img_file_path) {
        return Err("Failed to save image");
    }
    //Ok(obj_file_path)
    let python_command = Command::new("python")
    .arg("demo/demo.py")
    .arg("--config-file")
    .arg("configs/pix3d/meshrcnn_R50_FPN.yaml")
    .arg("--input")
    .arg("./input/input.png")
    .arg("--output")
    .arg("output_demo")
    .arg("--onlyhighest")
    .arg("MODEL.WEIGHTS")
    .arg("meshrcnn_S2_R50.pth")
    .current_dir("../../meshrnn/meshrcnn/")
    .output();

    match python_command {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("Command succeeded with output:\n{}", stdout);
            if !stderr.is_empty() {
                println!("Command had errors:\n{}", stderr);
            }
            Ok("success".to_string()) // 성공적으로 완료된 경우 Ok 반환
        },
        Err(_) => Err("fail"),
    }
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