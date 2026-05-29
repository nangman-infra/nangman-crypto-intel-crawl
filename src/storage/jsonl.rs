use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JsonlRecordLocator {
    pub(crate) line_number: usize,
    pub(crate) byte_offset: usize,
    pub(crate) byte_length: usize,
    pub(crate) content_sha256: String,
}

pub(crate) fn build_jsonl_chunk<T: Serialize>(
    records: &[T],
) -> Result<(Vec<u8>, Vec<JsonlRecordLocator>), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let mut locators = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let line = serde_json::to_vec(record)?;
        let byte_offset = bytes.len();
        let byte_length = line.len();
        let content_sha256 = format!("sha256:{}", hash_bytes(&line));
        bytes.extend_from_slice(&line);
        bytes.push(b'\n');
        locators.push(JsonlRecordLocator {
            line_number: index + 1,
            byte_offset,
            byte_length,
            content_sha256,
        });
    }
    Ok((bytes, locators))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Record {
        id: &'static str,
    }

    #[test]
    fn jsonl_chunk_tracks_record_locations() {
        let records = vec![Record { id: "a" }, Record { id: "b" }];

        let (bytes, locators) = build_jsonl_chunk(&records).unwrap();

        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "{\"id\":\"a\"}\n{\"id\":\"b\"}\n"
        );
        assert_eq!(locators[0].line_number, 1);
        assert_eq!(locators[0].byte_offset, 0);
        assert_eq!(locators[0].byte_length, "{\"id\":\"a\"}".len());
        assert_eq!(locators[1].line_number, 2);
        assert_eq!(locators[1].byte_offset, "{\"id\":\"a\"}\n".len());
    }
}
