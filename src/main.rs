use s3::{creds::Credentials, Bucket, Region};

extern crate s3;

struct Storage {
    name: String,
    region: Region,
    credentials: Credentials,
    bucket: String,
    location_supported: bool,
}

const MESSAGE: &str = "HELLo, World";

#[tokio::main]
async fn main() -> () {
    let minio = Storage {
        name: "minio".into(),
        region: Region::Custom {
            region: "minio".into(),
            endpoint: "http://127.0.0.1:9000".into(),
        },
        credentials: Credentials {
            access_key: Some("minio-admin".to_owned()),
            secret_key: Some("strongPassword".to_owned()),
            security_token: None,
            session_token: None,
        },
        bucket: "rusty-s3".to_string(),
        location_supported: false,
    };

    println!("Running: {}", minio.name);
    let bucket =
        Bucket::new_with_path_style(&minio.bucket, minio.region, minio.credentials).unwrap();

    let (_, code) = bucket
        .put_object("hello_world.txt", MESSAGE.as_bytes())
        .await
        .unwrap();
}
