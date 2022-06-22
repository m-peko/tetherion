/// Copyright (c) 2022 Tetherion

use {
    crate::block::Block,
    std::{fmt, result}
};

#[derive(Debug)]
pub enum InvalidBlockError {
    InvalidBlockId { id: u64, previous_id: u64 },
    InvalidPreviousHash { id: u64 },
    InvalidDifficulty { id: u64, difficulty: usize }
}

impl fmt::Display for InvalidBlockError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InvalidBlockError::InvalidBlockId { id, previous_id } =>
                write!(f, "Block with ID {} does not follow up previous block's ID {}", id, previous_id),
            InvalidBlockError::InvalidPreviousHash { id } =>
                write!(f, "Block with ID {} has the wrong previous hash", id),
            InvalidBlockError::InvalidDifficulty { id, difficulty } =>
                write!(f, "Block with ID {} does not satisfy difficulty of {}", id, difficulty)
        }
    }
}

impl std::error::Error for InvalidBlockError {}

#[derive(Debug, Clone)]
pub struct Tetherion<T: fmt::Display> {
    /// Blocks in the blockchain
    pub blocks: Vec<Block<T>>,

    /// The difficulty of the blockchain, i.e. measure of how difficult it is to mine a block
    pub difficulty: usize
}

impl<T: fmt::Display> Tetherion<T> {
    pub fn new(genesis_data: T, difficulty: usize) -> Self {
        let genesis = Block::<T>::genesis(genesis_data, difficulty);

        Self {
            blocks: vec![genesis],
            difficulty: difficulty
        }
    }

    /// Adds a new block to the blockchain
    pub fn add_block(&mut self, block: Block<T>) -> result::Result<(), InvalidBlockError> {
        let previous_block = self.blocks.last().expect("There should be at least one block in the blockchain!");
        match Tetherion::<T>::is_valid_block(previous_block, &block, self.difficulty) {
            Ok(()) => {
                self.blocks.push(block);
                Ok(())
            },
            Err(err) => return Err(err)
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
                Err(err) => return Err(err)
            };
        }

        Ok(())
    }

    /// Checks if the block to be added is valid regarding the previous block in the blockchain
    fn is_valid_block(previous_block: &Block<T>, block: &Block<T>, difficulty: usize) -> result::Result<(), InvalidBlockError> {
        if block.id != previous_block.id + 1 {
            return Err(InvalidBlockError::InvalidBlockId{ id: block.id, previous_id: previous_block.id });
        } else if block.previous_hash != previous_block.hash {
            return Err(InvalidBlockError::InvalidPreviousHash{ id: block.id });
        } else if !block.is_valid(difficulty) {
            return Err(InvalidBlockError::InvalidDifficulty{ id: block.id, difficulty: difficulty });
        }
        Ok(())
    }
}
