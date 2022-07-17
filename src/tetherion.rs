/// Copyright (c) 2022 Tetherion
use {
    crate::block::Block,
    serde::{Deserialize, Serialize},
    std::{fmt, result},
};

#[derive(Debug)]
pub enum InvalidBlockError {
    InvalidBlockId { id: u64, previous_id: u64 },
    InvalidPreviousHash { id: u64 },
    InvalidDifficulty { id: u64, difficulty: usize },
}

impl fmt::Display for InvalidBlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InvalidBlockError::InvalidBlockId { id, previous_id } => write!(
                f,
                "Block with ID {} does not follow up previous block's ID {}",
                id, previous_id
            ),
            InvalidBlockError::InvalidPreviousHash { id } => {
                write!(f, "Block with ID {} has the wrong previous hash", id)
            }
            InvalidBlockError::InvalidDifficulty { id, difficulty } => write!(
                f,
                "Block with ID {} does not satisfy difficulty of {}",
                id, difficulty
            ),
        }
    }
}

impl std::error::Error for InvalidBlockError {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tetherion<T: fmt::Display> {
    /// Blocks in the blockchain
    blocks: Vec<Block<T>>,

    /// The difficulty of the blockchain, i.e. measure of how difficult it is to mine a block
    difficulty: usize,
}

impl<T: fmt::Display> Tetherion<T> {
    pub fn new(genesis_data: T, difficulty: usize) -> Self {
        let genesis = Block::<T>::genesis(genesis_data, difficulty);

        Self {
            blocks: vec![genesis],
            difficulty: difficulty,
        }
    }

    /// Gets all the blocks of the blockchain
    pub fn blocks(&self) -> &Vec<Block<T>> {
        &self.blocks
    }

    /// Gets the blockchain's difficulty
    pub fn difficulty(&self) -> usize {
        self.difficulty
    }

    /// Gets the blockchain's creation timestamp
    pub fn creation_timestamp(&self) -> i64 {
        let genesis_block = self
            .blocks
            .first()
            .expect("There should be at least genesis block in the blockchain!");
        genesis_block.timestamp()
    }

    /// Adds a new block to the blockchain
    pub fn add_block(&mut self, block: Block<T>) -> result::Result<(), InvalidBlockError> {
        let previous_block = self
            .blocks
            .last()
            .expect("There should be at least one block in the blockchain!");
        match Tetherion::<T>::is_valid_block(previous_block, &block, self.difficulty) {
            Ok(()) => {
                self.blocks.push(block);
                Ok(())
            }
            Err(err) => return Err(err),
        }
    }

    /// Checks if blockchain is valid by validating each of the blocks regarding the previous block
    pub fn is_valid(&self) -> result::Result<(), InvalidBlockError> {
        // Blockchain has at least genesis block
        debug_assert!(self.blocks.len() >= 1);

        for i in 1..self.blocks.len() {
            let previous_block = self.blocks.get(i - 1).expect("Block should exist!");
            let current_block = self.blocks.get(i).expect("Block should exist!");

            match Tetherion::<T>::is_valid_block(previous_block, current_block, self.difficulty) {
                Ok(()) => continue,
                Err(err) => return Err(err),
            };
        }

        Ok(())
    }

    /// Checks if the block to be added is valid regarding the previous block in the blockchain
    fn is_valid_block(
        previous_block: &Block<T>,
        block: &Block<T>,
        difficulty: usize,
    ) -> result::Result<(), InvalidBlockError> {
        if block.id != previous_block.id + 1 {
            return Err(InvalidBlockError::InvalidBlockId {
                id: block.id,
                previous_id: previous_block.id,
            });
        } else if block.previous_hash != previous_block.hash {
            return Err(InvalidBlockError::InvalidPreviousHash { id: block.id });
        } else if !block.is_valid(difficulty) {
            return Err(InvalidBlockError::InvalidDifficulty {
                id: block.id,
                difficulty: difficulty,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        const DIFFICULTY: usize = 2;
        const GENESIS_DATA: &str = "genesis_data";

        let tetherion = Tetherion::<String>::new(String::from(GENESIS_DATA), DIFFICULTY);

        assert_eq!(
            tetherion.blocks.len(),
            1,
            "Only genesis block should be present in the blockchain on its creation"
        );
        assert_eq!(tetherion.blocks.last().unwrap().data(), GENESIS_DATA);
    }
}
