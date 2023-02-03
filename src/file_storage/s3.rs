use aws_sdk_s3::{types::ByteStream, Client};
use color_eyre::eyre::{ensure, Result};

use super::errors::UploadError;

pub struct AwsS3Storage {
    client: Client,
}

const MAX_SIZE: usize = 1024 * 1024 * 20; // 20 MB

impl AwsS3Storage {
    #[allow(unused)]
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);

        Self { client }
    }

    #[allow(unused)]
    /// Gets the upload URL for the object
    fn get_url(&self, bucket_name: &str, key: &str) -> String {
        let region_name = self.client.conf().region().map(|r| r.as_ref()).unwrap_or("us-west-2");

        format!("https://{}.s3.{}.amazonaws.com/{}", bucket_name, region_name, key)
    }

    #[allow(unused)]
    /// Uploads a bytes vector to given bucket and key
    pub async fn upload(
        &self,
        bucket_name: &str,
        key: &str,
        content_type: &str,
        bytes: Vec<u8>,
    ) -> Result<String> {
        let file_size = bytes.len();

        ensure!(file_size != 0, UploadError::BadSize());
        ensure!(file_size < MAX_SIZE, UploadError::ExceedsMaxSize());

        let body = ByteStream::from(bytes);

        // // Upload response and if successful get the upload url
        let upload_response = self
            .client
            .put_object()
            .bucket(bucket_name)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map(|_| self.get_url(bucket_name, key))?;

        Ok(upload_response)
    }

    #[allow(unused)]
    /// Removes object at given key from the bucket
    pub async fn delete(&self, bucket_name: &str, key: &str) -> Result<()> {
        self.client.delete_object().bucket(bucket_name).key(key).send().await?;
        Ok(())
    }
}

pub async fn connect() -> AwsS3Storage {
    AwsS3Storage::new().await
}
