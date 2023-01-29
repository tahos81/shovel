use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client;

pub struct AwsS3Storage {
    client: Client,
}

impl AwsS3Storage {
    pub async fn new() -> Self {
        let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        let config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&config);

        Self { client }
    }
}
