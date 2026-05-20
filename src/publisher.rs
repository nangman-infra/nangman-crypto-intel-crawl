use bytes::Bytes;
use serde::Serialize;
use std::error::Error;

pub(crate) enum EventPublisher {
    Disabled,
    JetStream(Box<JetStreamPublisher>),
}

impl EventPublisher {
    pub(crate) async fn connect(
        nats_url: Option<&str>,
        nats_subject: &str,
        nats_stream: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let Some(nats_url) = nats_url else {
            return Ok(Self::Disabled);
        };
        Ok(Self::JetStream(Box::new(
            JetStreamPublisher::connect(nats_url, nats_subject, nats_stream).await?,
        )))
    }

    pub(crate) fn is_enabled(&self) -> bool {
        matches!(self, Self::JetStream(_))
    }

    pub(crate) async fn publish<T: Serialize>(
        &self,
        message_id: &str,
        payload: &T,
    ) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Disabled => Ok(()),
            Self::JetStream(publisher) => publisher.publish(message_id, payload).await,
        }
    }

    pub(crate) async fn flush(&self) -> Result<(), Box<dyn Error>> {
        match self {
            Self::Disabled => Ok(()),
            Self::JetStream(publisher) => publisher.flush().await,
        }
    }
}

pub(crate) struct JetStreamPublisher {
    client: async_nats::Client,
    jetstream: async_nats::jetstream::Context,
    subject: String,
    stream: String,
}

impl JetStreamPublisher {
    async fn connect(nats_url: &str, subject: &str, stream: &str) -> Result<Self, Box<dyn Error>> {
        let client = async_nats::connect(nats_url).await?;
        let jetstream = async_nats::jetstream::new(client.clone());
        Ok(Self {
            client,
            jetstream,
            subject: subject.to_owned(),
            stream: stream.to_owned(),
        })
    }

    async fn publish<T: Serialize>(
        &self,
        message_id: &str,
        payload: &T,
    ) -> Result<(), Box<dyn Error>> {
        let bytes = Bytes::from(serde_json::to_vec(payload)?);
        let message = async_nats::jetstream::message::PublishMessage::build()
            .message_id(message_id)
            .expected_stream(&self.stream)
            .payload(bytes);
        let ack = self
            .jetstream
            .send_publish(self.subject.clone(), message)
            .await?
            .await?;
        if ack.stream != self.stream {
            return Err(format!(
                "NATS JetStream ack stream mismatch: expected {}, got {}",
                self.stream, ack.stream
            )
            .into());
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Box<dyn Error>> {
        self.client.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_nats::jetstream::stream::Config;
    use serde::Serialize;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[derive(Serialize)]
    struct SmokePayload {
        schema_version: &'static str,
        event_id: String,
    }

    #[tokio::test]
    #[ignore = "requires reachable NATS JetStream; set NATS_SMOKE_URL"]
    async fn publishes_with_jetstream_ack_and_stable_message_id() -> Result<(), Box<dyn Error>> {
        let nats_url =
            std::env::var("NATS_SMOKE_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_owned());
        let unique_suffix = unique_test_suffix();
        let stream = std::env::var("NATS_SMOKE_STREAM")
            .unwrap_or_else(|_| format!("RAW_INTEL_SMOKE_{unique_suffix}"));
        let subject = std::env::var("NATS_SMOKE_SUBJECT")
            .unwrap_or_else(|_| format!("raw_intel_event.created.smoke.{unique_suffix}"));

        let client = async_nats::connect(&nats_url).await?;
        let jetstream = async_nats::jetstream::new(client);
        let mut stream_info = jetstream
            .get_or_create_stream(Config {
                name: stream.clone(),
                subjects: vec![subject.clone()],
                max_messages: 10,
                max_age: Duration::from_secs(300),
                duplicate_window: Duration::from_secs(120),
                ..Default::default()
            })
            .await?;

        let publisher = JetStreamPublisher::connect(&nats_url, &subject, &stream).await?;
        let event_id = format!("smoke-event-{unique_suffix}");
        let payload = SmokePayload {
            schema_version: "raw_intel_event_created_v2",
            event_id: event_id.clone(),
        };

        publisher.publish(&event_id, &payload).await?;
        publisher.publish(&event_id, &payload).await?;
        publisher.flush().await?;

        let info = stream_info.info().await?;
        assert_eq!(info.config.name, stream);
        assert_eq!(info.state.messages, 1);
        Ok(())
    }

    fn unique_test_suffix() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before Unix epoch")
            .as_nanos()
    }
}
