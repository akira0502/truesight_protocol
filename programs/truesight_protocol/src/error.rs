// use thiserror::Error;
use solana_program::program_error::ProgramError;


// #[derive(Error, Debug, Copy, Clone)]
pub enum ValidationError {
    /// Invalid instruction
    // #[error("Not Enough TSD Tokens")]
    NotEnoughTokens,
}

impl From<ValidationError> for ProgramError {
    fn from(e: ValidationError) -> Self {
        ProgramError::Custom(e as u32)
    }
}