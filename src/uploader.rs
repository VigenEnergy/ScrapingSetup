use anyhow::Result;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::Credentials;
use aws_config::Region;
use std::collections::HashSet;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct Uploader {
    client: Client,
    bucket: String,
    pending_files: Arc<Mutex<HashSet<String>>>,
}

impl Uploader {
    pub async fn new(bucket: String, region: Option<String>, endpoint: Option<String>) -> Result<Self> {
        let region = region.unwrap_or_else(|| "eu-central".to_string());
        
        let mut s3_config_builder = aws_sdk_s3::config::Builder::new()
            .region(Region::new(region))
            .behavior_version_latest();
        
        // For S3-compatible services like Hetzner Object Storage
        if let Some(endpoint_url) = endpoint {
            s3_config_builder = s3_config_builder
                .endpoint_url(endpoint_url)
                .force_path_style(true); // Required for most S3-compatible services
        }
        
        // Try custom S3_* env vars first, then fall back to AWS_* env vars
        let access_key = env::var("S3_ACCESS_KEY")
            .or_else(|_| env::var("AWS_ACCESS_KEY_ID"));
        let secret_key = env::var("S3_SECRET_KEY")
            .or_else(|_| env::var("AWS_SECRET_ACCESS_KEY"));
        
        if let (Ok(access), Ok(secret)) = (access_key, secret_key) {
            let credentials = Credentials::new(access, secret, None, None, "env");
            s3_config_builder = s3_config_builder.credentials_provider(credentials);
        } else {
            // Fall back to default AWS credential chain
            let shared_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            if let Some(credentials_provider) = shared_config.credentials_provider() {
                s3_config_builder = s3_config_builder.credentials_provider(credentials_provider);
            }
        }
        
        let client = Client::from_conf(s3_config_builder.build());
        
        Ok(Self {
            client,
            bucket,
            pending_files: Arc::new(Mutex::new(HashSet::new())),
        })
    }

    pub fn get_pending_files_handle(&self) -> Arc<Mutex<HashSet<String>>> {
        self.pending_files.clone()
    }

    pub async fn run(&self) {
        info!("Starting S3 uploader for bucket: {}", self.bucket);
        
        loop {
            sleep(Duration::from_secs(60)).await;
            
            let files_to_upload = {
                let mut pending = self.pending_files.lock().await;
                let files: Vec<String> = pending.drain().collect();
                files
            };

            if files_to_upload.is_empty() {
                continue;
            }

            info!("Uploading {} files to S3", files_to_upload.len());

            let mut failed_uploads = Vec::new();

            for file_path in files_to_upload {
                if let Err(e) = self.upload_file(&file_path).await {
                    warn!("Failed to upload {}: {:?}. Will retry in next cycle.", file_path, e);
                    failed_uploads.push(file_path);
                }
            }

            if !failed_uploads.is_empty() {
                let mut pending = self.pending_files.lock().await;
                for file_path in failed_uploads {
                    pending.insert(file_path);
                }
            }
        }
    }

    async fn upload_file(&self, file_path: &str) -> Result<()> {
        let path = Path::new(file_path);
        let relative_path = path.strip_prefix("data/")?.to_string_lossy();
        let key = format!("data/{}", relative_path);
        
        let body = aws_sdk_s3::primitives::ByteStream::from_path(path).await?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body)
            .send()
            .await?;

        info!("Uploaded {}", key);
        Ok(())
    }
}
