#[macro_use]
extern crate rocket;

use rocket::futures::stream::{repeat, StreamExt};
use rocket::response::stream::ByteStream;
use s3::{creds::Credentials, Bucket, Region};

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

#[get("/<name>")]
async fn get_file(name: String) -> ByteStream![Vec<u8>] {
    let _minio = get_minio();
    let bucket = get_bucket();

    ByteStream! {
        let mut i = 0;
        loop {
        match bucket.get_object_range(name.to_owned(), i, Some(i + 1024)).await {
            Ok((data, 200..=299)) => yield data,
            Ok((_data, _code)) => break,
            Err(_e) => break,
            }
        i += 1025;
    }
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![get_file])
}
