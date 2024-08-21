#[macro_use] extern crate rocket;

use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::serde::{Serialize, Deserialize};
use rocket::Data;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use rocket_multipart_form_data::{MultipartFormDataOptions, MultipartFormData, MultipartFormDataField};
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
use std::process::Command;
use std::fs;

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

    if let Err(_) = python_command {
        return Err("Failed to generate obj");
    }

    let scan_dir = Path::new("../../meshrnn/meshrcnn/output_demo/input"); // 스캔할 디렉토리 경로
    let target_dir = Path::new("uploads/"); // 이동할 디렉토리 경로

     // 디렉토리를 읽고 .obj 파일만 필터링
     let mut obj_files: Vec<PathBuf> = match fs::read_dir(scan_dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("obj"))
            .collect(),
        Err(_) => return Err("Failed to read directory"),
    };

    // .obj 파일이 존재하는지 확인
    if obj_files.is_empty() {
        return Err("No .obj files found in the directory.");
    }
    // 첫 번째 .obj 파일을 지정된 디렉토리로 이동
    let first_obj = obj_files.remove(0); // 첫 번째 파일 선택 및 목록에서 제거
    let target_path = target_dir.join(first_obj.file_name().unwrap());

    // 디렉토리가 존재하지 않으면 생성
    if !target_dir.exists() {
        if let Err(e) = fs::create_dir_all(target_dir){
            return Err("Fail to create output directory."); 
        }
    }

    if let Err(e) = fs::rename(&first_obj, &target_path){
        return Err("Fail to move to ouput directory."); 
    }

     // 나머지 파일 삭제
    if let Err(e) = fs::remove_dir_all(scan_dir) {
        println!("Failed to delete output dir : {}", e);
    }

    Ok(target_path.display().to_string())
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