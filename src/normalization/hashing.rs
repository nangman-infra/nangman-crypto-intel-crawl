use sha2::{Digest, Sha256};
use std::fmt::Write;

pub(crate) fn hash_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

pub(crate) fn simhash64(normalized_text: &str) -> u64 {
    let tokens = normalized_text.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return 0;
    }
    let features = shingles(&tokens);
    let mut weights = [0i32; 64];
    for feature in features {
        let hash = first_u64_hash(&feature);
        for (bit, weight) in weights.iter_mut().enumerate() {
            if hash & (1u64 << bit) == 0 {
                *weight -= 1;
            } else {
                *weight += 1;
            }
        }
    }
    weights.iter().enumerate().fold(0u64, |acc, (bit, weight)| {
        if *weight > 0 {
            acc | (1u64 << bit)
        } else {
            acc
        }
    })
}

pub(crate) fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

fn shingles(tokens: &[&str]) -> Vec<String> {
    if tokens.len() < 4 {
        return tokens.iter().map(|token| (*token).to_owned()).collect();
    }
    tokens
        .windows(4)
        .map(|window| window.join(" "))
        .collect::<Vec<_>>()
}

fn first_u64_hash(value: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    u64::from_be_bytes(bytes)
}
