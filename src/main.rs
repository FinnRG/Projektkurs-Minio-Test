#[macro_use]
extern crate rocket;

use regex::Regex;
use rocket::data::ToByteUnit;
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
    region: Region,
    credentials: Credentials,
    bucket: String,
}

fn get_minio() -> Storage {
    Storage {
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
    }
}

fn get_bucket() -> Bucket {
    let minio = get_minio();
    Bucket::new_with_path_style(&minio.bucket, minio.region, minio.credentials).unwrap()
}

#[get("/get/<name..>")]
async fn get_file(name: PathBuf) -> ByteStream![Vec<u8>] {
    let bucket = get_bucket();
    let mut name_str = name.into_os_string().into_string().unwrap();
    let re = Regex::new(r"\d*\.(ts|m3u8)$").unwrap();
    let name_stripped = re.replace(&name_str, "").into_owned();

    // Reroutes the /get/name to /get/name.m3u8 request
    if !(name_str.ends_with(".ts") || name_str.ends_with(".m3u8")) {
        name_str = name_str + ".m3u8";
    }

    let path_str = format!("media/{}/output/{}", name_stripped, name_str);

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
async fn upload(mut name: String, paste: Data<'_>) -> Result<String, Debug<std::io::Error>> {
    let re = Regex::new(r"\.\w*$").unwrap();
    name = re.replace(&name, "").into_owned();
    tokio::fs::create_dir_all(format!("media/{}/output", name)).await?;
    let filename = format!("media/{name}/{name}", name = name);
    paste.open(1u32.gibibytes()).into_file(&filename).await?;

    println!("{}", &filename);
    // Transform the given file into HLS streamable files
    let _output = Command::new("/usr/bin/ffmpeg")
        .args(&[
            "-i",
            &filename,
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
            &format!("./media/{name}/output/{name}.m3u8", name = name),
        ])
        .output()
        .expect("Failed to execute command");

    let bucket = get_bucket();
    let paths = fs::read_dir(format!("media/{name}/output/", name = name)).unwrap();

    for path in paths {
        let path_ex = path.unwrap().path();
        let temp_path = path_ex.to_str().unwrap();
        bucket
            .put_object(temp_path, &fs::read(temp_path).unwrap())
            .await
            .unwrap();
    }

    tokio::fs::remove_dir_all(format!("media/{name}", name = name)).await?;

    //String::from_utf8_lossy(&output.stdout).to_string()
    Ok(String::from("Successfull"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let allowed_origins = AllowedOrigins::some_exact(&["http://localhost:3000"]);

    let cors = rocket_cors::CorsOptions {
        allowed_origins,
        allowed_methods: vec![Method::Get, Method::Post]
            .into_iter()
            .map(From::from)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
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
