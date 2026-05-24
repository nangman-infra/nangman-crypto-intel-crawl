use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_types::region::Region;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObjectStoreConfig {
    pub(crate) bucket: String,
    pub(crate) region: String,
}

#[derive(Clone)]
pub(crate) struct ObjectStore {
    client: Client,
    bucket: String,
}

impl ObjectStore {
    pub(crate) async fn connect(config: ObjectStoreConfig) -> Result<Self, Box<dyn Error>> {
        validate_config(&config)?;
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .load()
            .await;
        let store = Self {
            client: Client::new(&sdk_config),
            bucket: config.bucket,
        };
        store.head_bucket().await?;
        Ok(store)
    }

    pub(crate) fn bucket(&self) -> &str {
        &self.bucket
    }

    pub(crate) async fn head_bucket(&self) -> Result<(), Box<dyn Error>> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await?;
        Ok(())
    }

    pub(crate) async fn put_bytes(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(bytes))
            .send()
            .await?;
        Ok(())
    }

    pub(crate) async fn get_bytes(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        Ok(output.body.collect().await?.into_bytes().to_vec())
    }

    pub(crate) async fn key_exists(&self, key: &str) -> Result<bool, Box<dyn Error>> {
        Ok(!self.list_keys(key).await?.is_empty())
    }

    pub(crate) async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut keys = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(prefix);
            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }
            let output = request.send().await?;
            for object in output.contents() {
                if let Some(key) = object.key() {
                    keys.push(key.to_owned());
                }
            }
            if !output.is_truncated().unwrap_or(false) {
                break;
            }
            let Some(token) = output.next_continuation_token() else {
                break;
            };
            continuation_token = Some(token.to_owned());
        }

        keys.sort();
        Ok(keys)
    }
}

fn validate_config(config: &ObjectStoreConfig) -> Result<(), Box<dyn Error>> {
    if config.bucket.trim().is_empty() {
        return Err("object store bucket is required".into());
    }
    if config.bucket.contains('<') || config.bucket.contains('>') {
        return Err(
            "object store bucket must be a real AWS S3 bucket name, not a placeholder".into(),
        );
    }
    if config.region.trim().is_empty() {
        return Err("object store region is required".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_config_values() {
        let error = validate_config(&ObjectStoreConfig {
            bucket: "".to_owned(),
            region: "us-east-1".to_owned(),
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("bucket"));
    }

    #[test]
    fn rejects_bucket_placeholder() {
        let error = validate_config(&ObjectStoreConfig {
            bucket: "<bucket-name>".to_owned(),
            region: "ap-northeast-2".to_owned(),
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("placeholder"));
    }
}
