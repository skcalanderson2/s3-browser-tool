use error::S3ExampleError;
pub mod error;

pub async fn list_objects_keys(
    client: &aws_sdk_s3::Client,
    bucket: &str,
) -> Result<Vec<String>, S3ExampleError> {
    let mut keys = Vec::new();
    let mut response = client
        .list_objects_v2()
        .bucket(bucket.to_owned())
        .into_paginator()
        .send();

    while let Some(result) = response.next().await {
        let output = result.map_err(S3ExampleError::from)?;
        for object in output.contents() {
            if let Some(key) = object.key() {
                keys.push(key.to_string());
            }
        }
    }

    Ok(keys)
}

pub async fn list_objects(client: &aws_sdk_s3::Client, bucket: &str) -> Result<(), S3ExampleError> {
    let mut response = client
        .list_objects_v2()
        .bucket(bucket.to_owned())
        .max_keys(10)
        .into_paginator()
        .send();

    while let Some(result) = response.next().await {
        match result {
            Ok(output) => {
                for object in output.contents() {
                    println!(" - {}", object.key().unwrap_or("Unknown"));
                }
            }
            Err(err) => {
                eprintln!("{err:?}")
            }
        }
    }

    Ok(())
}

pub async fn upload_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    file_name: &str,
    key: &str,
) -> Result<aws_sdk_s3::operation::put_object::PutObjectOutput, S3ExampleError> {
    let body = aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(file_name)).await;
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body.unwrap())
        .send()
        .await
        .map_err(S3ExampleError::from)
}

pub async fn remove_object(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
) -> Result<(), S3ExampleError> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;

    // There are no modeled errors to handle when deleting an object.

    Ok(())
}
