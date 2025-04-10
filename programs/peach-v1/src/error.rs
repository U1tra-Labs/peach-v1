use anchor_lang::prelude::*;
use core::fmt::Display;

#[error_code]
pub enum PeachError {
    #[msg("")]
    SomeError,
    #[msg("instruction is disabled")]
    IxIsDisabled,
    #[msg("an oracle does not reach the confidence threshold")]
    OracleConfidence,
    #[msg("an oracle is stale")]
    OracleStale,
    #[msg("deposit crosses the current market deposit limit")]
    DepositLimit,
    #[msg("token is in reduce only mode")]
    TokenInReduceOnlyMode,
    #[msg("token deposits into accounts that are being liquidated must bring their health above the init threshold")]
    DepositsIntoLiquidatingMustRecover,
    #[msg("token position does not exist")]
    TokenPositionDoesNotExist,
    #[msg("")]
    NotImplementedError,
    #[msg("oracle type cannot be determined")]
    UnknownOracleType,
    #[msg("incorrect number of health accounts")]
    InvalidHealthAccountCount,
    #[msg("invalid bank")]
    InvalidBank,
    #[msg("account is frozen")]
    AccountIsFrozen,
    #[msg("no free token position index")]
    NoFreeTokenPositionIndex,
    #[msg("deposit crosses the token's deposit limit")]
    BankDepositLimit,
    #[msg("")]
    UnexpectedOracle,
    #[msg("bank vault has insufficent funds")]
    InsufficentBankVaultFunds,
    #[msg("delegates can only withdraw to the owner's associated token account")]
    DelegateWithdrawOnlyToOwnerAta,
    #[msg("delegates can only withdraw if they close the token position")]
    DelegateWithdrawMustClosePosition,
    #[msg("for borrows the bank must be in the health account list")]
    BorrowsRequireHealthAccountBank,
    #[msg("account is currently being liquidated")]
    BeingLiquidated,
    #[msg("health must be positive or not decrease")]
    HealthMustBePositiveOrIncrease, // outdated name is kept for backwards compatibility
    #[msg("health must be positive")]
    HealthMustBePositive,
    #[msg("bank utilization has reached limit")]
    BankBorrowLimitReached,
    #[msg("bank net borrows has reached limit - this is an intermittent error - the limit will reset regularly")]
    BankNetBorrowsLimitReached,
    #[msg("Init Asset Weight can't be negative")]
    InitAssetWeightCantBeNegative,
    #[msg("Invalid Kamino user metadata account provided.")]
    InvalidKaminoUserMetadataAccount,
    #[msg("Invalid deposit amount: must be greater than zero.")]
    InvalidAmount,
    #[msg("Invalid Kamino reserve liquidity supply account provided.")]
    InvalidKaminoReserveLiquiditySupplyAccount,
    #[msg("Wrong Lending Protocol Index for this Instruction.")]
    LendingProtocolMismatch,
    #[msg("Deposit Information Not Found.")]
    DepositNotFound
}

impl PeachError {
    pub fn error_code(&self) -> u32 {
        (*self).into()
    }
}

pub trait IsAnchorErrorWithCode {
    fn is_anchor_error_with_code(&self, code: u32) -> bool;
    fn is_oracle_error(&self) -> bool;
}

impl<T> IsAnchorErrorWithCode for anchor_lang::Result<T> {
    fn is_anchor_error_with_code(&self, code: u32) -> bool {
        match self {
            Err(Error::AnchorError(error)) => error.error_code_number == code,
            _ => false,
        }
    }
    fn is_oracle_error(&self) -> bool {
        match self {
            Err(Error::AnchorError(e)) => {
                e.error_code_number == PeachError::OracleConfidence.error_code()
                    || e.error_code_number == PeachError::OracleStale.error_code()
            }
            _ => false,
        }
    }
}

pub trait Contextable {
    /// Add a context string `c` to a Result or Error
    ///
    /// Example: foo().context("calling foo")?;
    fn context(self, c: impl Display) -> Self;

    /// Like `context()`, but evaluate the context string lazily
    ///
    /// Use this if it's expensive to generate, like a format!() call.
    fn with_context<C, F>(self, c: F) -> Self
    where
        C: Display,
        F: FnOnce() -> C;
}

impl Contextable for Error {
    fn context(self, c: impl Display) -> Self {
        match self {
            Error::AnchorError(err) => Error::AnchorError(Box::new(AnchorError {
                error_msg: if err.error_msg.is_empty() {
                    format!("{}", c)
                } else {
                    format!("{}; {}", err.error_msg, c)
                },
                ..*err
            })),
            // Maybe wrap somehow?
            Error::ProgramError(err) => Error::ProgramError(err),
        }
    }
    fn with_context<C, F>(self, c: F) -> Self
    where
        C: Display,
        F: FnOnce() -> C,
    {
        self.context(c())
    }
}

impl<T> Contextable for Result<T> {
    fn context(self, c: impl Display) -> Self {
        if let Err(err) = self {
            Err(err.context(c))
        } else {
            self
        }
    }
    fn with_context<C, F>(self, c: F) -> Self
    where
        C: Display,
        F: FnOnce() -> C,
    {
        if let Err(err) = self {
            Err(err.context(c()))
        } else {
            self
        }
    }
}

/// Creates an Error with a particular message, using format!() style arguments
///
/// Example: error_msg!("index {} not found", index)
#[macro_export]
macro_rules !error_msg {
    ($($arg:tt)*) => {
        error!(PeachError::SomeError).context(format!($($arg)*))
    };
}

/// Creates an Error with a particular message, using format!() style arguments
///
/// Example: error_msg_typed!(TokenPositionMissing, "index {} not found", index)
#[macro_export]
macro_rules !error_msg_typed {
    ($code:expr, $($arg:tt)*) => {
        error!($code).context(format!($($arg)*))
    };
}

/// Like anchor's require!(), but with a customizable message
///
/// Example: require_msg!(condition, "the condition on account {} was violated", account_key);
#[macro_export]
macro_rules !require_msg {
    ($invariant:expr, $($arg:tt)*) => {
        if !($invariant) {
            return Err(error_msg!($($arg)*));
        }
    };
}

/// Like anchor's require!(), but with a customizable message and type
///
/// Example: require_msg_typed!(condition, "the condition on account {} was violated", account_key);
#[macro_export]
macro_rules !require_msg_typed {
    ($invariant:expr, $code:expr, $($arg:tt)*) => {
        if !($invariant) {
            return Err(error_msg_typed!($code, $($arg)*));
        }
    };
}

pub use error_msg;
pub use error_msg_typed;
pub use require_msg;
pub use require_msg_typed;
