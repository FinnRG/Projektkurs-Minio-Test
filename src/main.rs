#[macro_use]
extern crate rocket;

use rocket::data::ToByteUnit;
use rocket::futures::stream::{repeat, StreamExt};
use rocket::http::Method;
use rocket::response::stream::ByteStream;
use rocket::response::Debug;
use rocket::{get, routes, Data};
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use s3::{creds::Credentials, Bucket, Region};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

extern crate s3;

struct Storage {
    name: String,
    region: Region,
    credentials: Credentials,
    bucket: String,
    location_supported: bool,
}

const MESSAGE: &str = "Hello, World!";

fn get_minio() -> Storage {
    Storage {
        name: "minio".into(),
        region: Region::Custom {
            region: "minio".into(),
            endpoint: "http://localhost:9000".into(),
        },
        credentials: Credentials {
            access_key: Some("minio-admin".to_owned()),
            secret_key: Some("strongPassword".to_owned()),
            security_token: None,
            session_token: None,
        },
        bucket: "rusty-s3".to_string(),
        location_supported: false,
    }
}

fn get_bucket() -> Bucket {
    let minio = get_minio();
    Bucket::new_with_path_style(&minio.bucket, minio.region, minio.credentials).unwrap()
}

async fn put_file(name: &str) -> () {
    let bucket = get_bucket();

    let (_, code) = bucket
        .put_object("hello_worldd.txt", MESSAGE.as_bytes())
        .await
        .unwrap();
}

#[get("/get/<name..>")]
async fn get_file(name: PathBuf) -> ByteStream![Vec<u8>] {
    let _minio = get_minio();
    let bucket = get_bucket();

    let path_str = name.into_os_string().into_string().unwrap();

    ByteStream! {
        let mut i = 0;
        loop {
        match bucket.get_object_range(path_str.clone(), i, Some(i + 1024 * 1024)).await {
            Ok((data, 200..=299)) => yield data,
            Ok((_data, _code)) => break,
            Err(_e) => break,
            }
        i += 1024 * 1024 + 1;
    }
    }
}

#[post("/upload/<name>", data = "<paste>")]
async fn upload(name: String, paste: Data<'_>) -> Result<String, Debug<std::io::Error>> {
    let filename = format!("media/{name}", name = name);
    paste.open(1u32.gibibytes()).into_file(filename).await?;

    println!("{}", format!("./media/{}", name));

    // Transform the given file into HLS streamable files
    let _output = Command::new("/usr/bin/ffmpeg")
        .args(&[
            "-i",
            &format!("./media/{}", name),
            "-codec:",
            "copy",
            "-start_number",
            "0",
            "-hls_time",
            "10",
            "-hls_list_size",
            "0",
            "-f",
            "hls",
            &format!("./media/output/{}.m3u8", name),
        ])
        .output()
        .expect("Failed to execute command");

    let paths = fs::read_dir("media/output").unwrap();
    let bucket = get_bucket();

    for path in paths {
        let path_ex = path.unwrap().path();
        let temp_path = path_ex.to_str().unwrap();
        let (_, code) = bucket
            .put_object(temp_path, &fs::read(temp_path).unwrap())
            .await
            .unwrap();
    }

    //String::from_utf8_lossy(&output.stdout).to_string()
    Ok(String::from("Test"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let allowed_origins = AllowedOrigins::some_exact(&["http://localhost:3000"]);

    // You can also deserialize this
    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
        allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()?;
    rocket::build()
        .mount("/", routes![get_file, upload])
        .attach(cors)
        .launch()
        .await?;

    Ok(())
}
