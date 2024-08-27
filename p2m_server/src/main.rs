#[macro_use] extern crate rocket;

use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::Data;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use rocket_multipart_form_data::{MultipartFormDataOptions, MultipartFormData, MultipartFormDataField};
use image::io::Reader as ImageReader;
use std::process::Command;
use std::fs;
use rocket::response::Responder;
use rocket::{Request, Response};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, ImageOutputFormat};
use image::imageops::{grayscale, rotate90, rotate180, rotate270};

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

#[post("/upload/obj", data = "<data>")]
async fn uploadObj(content_type: &ContentType, data: Data<'_>) -> Result<String, &'static str> {
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
        if let Err(_) = fs::create_dir_all(target_dir){
            return Err("Fail to create output directory. "); 
        }
    }

    if let Err(_) = fs::rename(&first_obj, &target_path){
        return Err("Fail to move to ouput directory. "); 
    }

     // 나머지 파일 삭제
    if let Err(e) = fs::remove_dir_all(scan_dir) {
        println!("Failed to delete output dir : {}", e);
    }

    Ok(target_path.display().to_string())
}

#[get("/download/obj/<file..>")]
async fn downloadObj(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("uploads/").join(file)).await.ok()
}

#[post("/upload/img", data = "<data>")]
async fn upload(content_type: &ContentType, data: Data<'_>) -> Result<String, &'static str> {
    let options = MultipartFormDataOptions::with_multipart_form_data_fields(vec![
        MultipartFormDataField::file("file").size_limit(10 * 1024 * 1024), // 10MB 제한
    ]);

    let multipart_form_data = match MultipartFormData::parse(content_type, data, options).await {
        Ok(form) => form,
        Err(_) => return Err("Failed to parse form data"),
    };

    if let Some(file_field) = multipart_form_data.files.get("file") {
        if let Some(file) = file_field.get(0) {
            let file_name = file.file_name.as_deref().unwrap_or("upload"); // 파일 이름
            let file_path = format!("./uploads/{}", file_name); // 저장할 경로

            // 파일을 저장할 경로로 복사
            match rocket::tokio::fs::copy(&file.path, &file_path).await {
                Ok(_) => return Ok("File uploaded successfully".to_string()),
                Err(_) => return Err("Failed to save file"),
            }
        }
    }

    Err("File field is missing")
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ImageProcessRequest {
    grayscale: u32,   // 0~100
    brightness: u32,  // 0~100
    threshold: u32,   // 0~100
    rotation: u32,    // 0, 90, 180, 270 degrees
}

struct ProcessedImage {
    content_type: ContentType,
    data: Vec<u8>,
}

impl<'r> Responder<'r, 'static> for ProcessedImage {
    fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
        let size = self.data.len();
        Response::build()
            .header(self.content_type)
            .sized_body(size, Cursor::new(self.data))
            .ok()
    }
}

#[get("/download/img/<file..>", format = "json", data = "<request>")]
async fn download(file: PathBuf, request: Json<ImageProcessRequest>) -> Result<ProcessedImage, &'static str> {
    let request = request.into_inner();
    let img_path = Path::new("uploads/").join(file);

    // 이미지 로드
    let mut img = match image::open(&img_path) {
        Ok(img) => img,
        Err(_) => return Err("Failed to open the image"),
    };

    // 그레이스케일 처리
    if request.grayscale > 0 {
        img = apply_grayscale(&img, request.grayscale);
    }

    // 밝기 조정
    if request.brightness != 50 {
        let brightness_factor = request.brightness as f32 / 50.0;
        img = adjust_brightness(&img, brightness_factor);
    }

    // 임계값 처리
    if request.threshold > 0 {
        let threshold = request.threshold as f32 * 2.5;
        img = apply_threshold(&img, threshold);
    }

    // 회전 처리
    img = match request.rotation {
        90 => image::DynamicImage::ImageRgba8(rotate90(&img)),
        180 => image::DynamicImage::ImageRgba8(rotate180(&img)),
        270 => image::DynamicImage::ImageRgba8(rotate270(&img)),
        _ => img,
    };

    // 이미지를 PNG 포맷으로 저장
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageOutputFormat::Png)
        .map_err(|_| "Failed to encode image")?;

    Ok(ProcessedImage {
        content_type: ContentType::PNG,
        data: buf,
    })
}

// 그레이스케일 적용 함수
fn apply_grayscale(img: &DynamicImage, grayscale_factor: u32) -> DynamicImage {
    let gray_img = grayscale(img);
    let factor = grayscale_factor as f32 / 100.0;

    let (width, height) = gray_img.dimensions();
    let mut output_img = ImageBuffer::new(width, height);

    for (x, y, pixel) in gray_img.enumerate_pixels() {
        let gray_value = pixel[0] as f32 * factor;
        let [r, g, b, a] = img.get_pixel(x, y).0;
        output_img.put_pixel(x, y, Rgba([
            (r as f32 * (1.0 - factor) + gray_value) as u8,
            (g as f32 * (1.0 - factor) + gray_value) as u8,
            (b as f32 * (1.0 - factor) + gray_value) as u8,
            a,
        ]));
    }

    DynamicImage::ImageRgba8(output_img)
}

// 밝기 조정 함수
fn adjust_brightness(img: &DynamicImage, brightness_factor: f32) -> DynamicImage {
    let (width, height) = img.dimensions();
    let mut output_img = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.pixels() {
        let [r, g, b, a] = pixel.0;
        let adjust = |c: u8| (c as f32 * brightness_factor).clamp(0.0, 255.0) as u8;
        output_img.put_pixel(x, y, Rgba([adjust(r), adjust(g), adjust(b), a]));
    }

    DynamicImage::ImageRgba8(output_img)
}

// 임계값 적용 함수
fn apply_threshold(img: &DynamicImage, threshold: f32) -> DynamicImage {
    let (width, height) = img.dimensions();
    let mut output_img = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.pixels() {
        let [r, g, b, a] = pixel.0;
        let apply_threshold = |c: u8| if c as f32 >= threshold { c } else { 0 };
        output_img.put_pixel(x, y, Rgba([
            apply_threshold(r),
            apply_threshold(g),
            apply_threshold(b),
            a,
        ]));
    }

    DynamicImage::ImageRgba8(output_img)
}


#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, uploadObj, downloadObj, upload, download])
}