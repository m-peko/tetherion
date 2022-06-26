/// Copyright (c) 2022 Tetherion

use {
    serde::{Deserialize, Serialize},
    sha2::{Digest, Sha256}
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block<T: std::fmt::Display> {
    /// The ID indicating the position of the block in the blockchain
    pub id: u64,

    /// The hash value of the block in the blockchain
    pub hash: String,

    /// The hash value of the previous block in the blockchain
    pub previous_hash: String,

    /// The timestamp of when the block was created
    timestamp: i64,

    /// The nonce calculated by the Proof of Work consensus algorithm
    nonce: u64,

    /// The data stored in the block
    data: T
}

impl<T: std::fmt::Display> Block<T> {
    pub fn new(id: u64, previous_hash: &str, data: T, difficulty: usize) -> Self {
        let mut block = Self {
            id: id,
            hash: String::from(""),
            previous_hash: String::from(previous_hash),
            timestamp: chrono::Utc::now().timestamp(),
            nonce: 0,
            data: data
        };

        block.mine(difficulty);
        block
    }

    /// Gets the data contained in the block
    #[allow(dead_code)]
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Creates a genesis block
    pub fn genesis(data: T, difficulty: usize) -> Self {
        Block::<T>::new(0, "genesis", data, difficulty)
    }

    /// Checks if block's hash has the specified difficulty
    pub fn is_valid(&self, difficulty: usize) -> bool {
        const HEX_SIZE: usize = 2;

        let pattern = &"0".repeat(difficulty * HEX_SIZE);
        self.hash.starts_with(pattern)
    }

    /// Mines a block by producing a valid nonce and the block's hash
    fn mine(&mut self, difficulty: usize) {
        log::info!("Mining the block...");

        if self.nonce != 0 {
            panic!("Block should be mined only once, at its creation time");
        }

        loop {
            self.hash = hex::encode(Block::<T>::hash(self.hash_data().as_bytes()));
            if self.is_valid(difficulty) {
                log::info!("Valid nonce found: {}", self.nonce);
                break;
            }

            // Try the next nonce
            self.nonce += 1;
        }
    }

    /// Creates the input for the hash algorithm
    fn hash_data(&self) -> String {
        let mut hash_data = self.id.to_string();
        hash_data.push_str(&self.previous_hash);
        hash_data.push_str(&self.timestamp.to_string());
        hash_data.push_str(&self.nonce.to_string());
        hash_data.push_str(&self.data.to_string());
        hash_data
    }

    /// Creates a SHA256 hash value in HEX format from raw bytes
    fn hash(data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().as_slice().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid() {
        const VALID_DIFFICULTY: usize = 2;
        const INVALID_DIFFICULTY: usize = 3;

        let block = Block::<String>::new(
            0,
            "some_previous_hash",
            String::from("data"),
            VALID_DIFFICULTY
        );

        assert!(block.is_valid(VALID_DIFFICULTY));
        assert!(!block.is_valid(INVALID_DIFFICULTY));
    }

    #[test]
    #[should_panic(expected = "Block should be mined only once, at its creation time")]
    fn mine_multiple_times() {
        const DIFFICULTY: usize = 2;

        let mut block = Block::<String>::new(
            0,
            "some_previous_hash",
            String::from("data"),
            DIFFICULTY
        );

        block.mine(DIFFICULTY);
    }
}