use std::cell::Ref;
use std::cell::RefMut;
use std::mem::size_of;

use anchor_lang::zero_copy;
use anchor_lang::prelude::*;
use anchor_lang::Discriminator;
use arrayref::array_ref;
use fixed::types::I80F48;
use anchor_lang::solana_program::program_memory::sol_memmove;
use static_assertions::const_assert_eq;
use crate::error::Contextable;
use crate::error::PeachError;
use crate::error_msg_typed;
use crate::custom_types::fixed_wrapper::FixedWrapper;
use crate::health::HealthCache;
use crate::health::HealthType;
use crate::logs::emit_stack;
use crate::logs::DeactivateTokenPositionLog;
use crate::state::*;

use crate::custom_types::TokenIndex;

type BorshVecLength = u32;
const BORSH_VEC_PADDING_BYTES: usize = 4;
const BORSH_VEC_SIZE_BYTES: usize = 4;
const DEFAULT_PEACH_ACCOUNT_VERSION: u8 = 1;
const DYNAMIC_RESERVED_BYTES: usize = 64;

pub struct PeachAccountPdaSeeds {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub account_num_bytes: [u8; 4],
    pub bump_bytes: [u8; 1],
}

impl PeachAccountPdaSeeds {
    pub fn signer_seeds(&self) -> [&[u8]; 5] {
        [
            b"PeachAccount".as_ref(),
            self.market.as_ref(),
            self.owner.as_ref(),
            &self.account_num_bytes,
            &self.bump_bytes,
        ]
    }
}

// Peach Account
// This struct definition is only for clients e.g. typescript, so that they can easily use out of the box
// deserialization and not have to do custom deserialization
// On chain, we would prefer zero-copying to optimize for compute
//
// The PeachAccount binary data has changed over time:
// - v1: The original version, many mainnet accounts still are this version.
//       The PeachAccount struct below describes v1 to make sure reading by IDL works for all live
//       accounts.
// - v2: Introduced in v0.18.0 to add token conditional swaps at the end. Users using account
//       resizing before v0.20.0 would migrate to this version.
// - v3: Introduced in v0.20.0 to add 64 zero bytes at the end for future expansion.
//       Users will migrate to this version when resizing their accounts. Also the
//       AccountSizeMigration instruction was used to bring all accounts to
//       this version after v0.20.0 was deployed.
//
// Version v0.22.0 drops idl support for v1 and v2 accounts by extending the PeachAccount idl with the
// new fields.
//
// When not reading via idl, PeachAccount binary data is backwards compatible: when ignoring trailing bytes,
// a v2 account can be read as a v1 account and a v3 account can be read as v1 or v2 etc.
#[account]
#[derive(Debug, PartialEq)]
pub struct PeachAccount {
    // fixed
    // note: keep PeachAccountFixed in sync with changes here
    // ABI: Clients rely on this being at offset 8
    pub market: Pubkey,

    // ABI: Clients rely on this being at offset 40
    pub owner: Pubkey,

    pub name: [u8; 32],

    // Alternative authority/signer of transactions for a peach account
    pub delegate: Pubkey,

    pub account_num: u32,

    /// Tracks that this account should be liquidated until init_health >= 0.
    ///
    /// Normally accounts can not be liquidated while maint_health >= 0. But when an account
    /// reaches maint_health < 0, liquidators will call a liquidation instruction and thereby
    /// set this flag. Now the account may be liquidated until init_health >= 0.
    ///
    /// Many actions should be disabled while the account is being liquidated, even if
    /// its maint health has recovered to positive. Creating new open orders would, for example,
    /// confuse liquidators.
    pub being_liquidated: u8,

    /// The account is currently inside a health region marked by HealthRegionBegin...HealthRegionEnd.
    ///
    /// Must never be set after a transaction ends.
    pub in_health_region: u8,

    pub bump: u8,

    pub sequence_number: u8,

    // (Display only)
    // Cumulative (deposits - withdraws)
    // using USD prices at the time of the deposit/withdraw
    // in USD units with 6 decimals
    pub net_deposits: i64,
    // (Display only)
    // Cumulative transfers from perp to spot positions
    // pub perp_spot_transfers: i64,

    /// Init health as calculated during HealthReginBegin, rounded up.
    pub health_region_begin_init_health: i64,

    pub frozen_until: u64,

    /// Fees usable with the "fees buyback" feature.
    /// This tracks the ones that accrued in the current expiry interval.
    pub buyback_fees_accrued_current: u64,
    /// Fees buyback amount from the previous expiry interval.
    pub buyback_fees_accrued_previous: u64,
    /// End timestamp of the current expiry interval of the buyback fees amount.
    pub buyback_fees_expiry_timestamp: u64,

    /// Next id to use when adding a token condition swap
    // pub next_token_conditional_swap_id: u64,

    pub temporary_delegate: Pubkey,
    pub temporary_delegate_expiry: u64,

    /// Time at which the last collateral fee was charged
    pub last_collateral_fee_charge: u64,

    pub reserved: [u8; 152],

    // dynamic
    pub header_version: u8,
    pub padding3: [u8; 7],
    // note: padding is required for TokenPosition, etc. to be aligned
    pub padding4: u32,
    // Maps token_index -> deposit/borrow account for each token
    // that is active on this PeachAccount.
    pub tokens: Vec<TokenPosition>,
    pub padding5: u32,
    // Maps serum_market_index -> open orders for each serum market
    // that is active on this PeachAccount.
    // pub serum3: Vec<Serum3Orders>,
    // #[derivative(Debug = "ignore")]
    // pub padding6: u32,
    // pub perps: Vec<PerpPosition>,
    // #[derivative(Debug = "ignore")]
    // pub padding7: u32,
    // pub perp_open_orders: Vec<PerpOpenOrder>,
    // #[derivative(Debug = "ignore")]
    // pub padding8: u32,
    // pub token_conditional_swaps: Vec<TokenConditionalSwap>,

    pub reserved_dynamic: [u8; 64],
}

impl PeachAccount {

    /// Number of bytes needed for the PeachAccount, including the discriminator
    pub fn space(token_count: u8) -> usize {
        8 + size_of::<PeachAccountFixed>()
            + Self::dynamic_size(token_count)
    }

    pub fn dynamic_token_vec_offset() -> usize {
        8 // header version + padding
            + BORSH_VEC_PADDING_BYTES 
    }

    pub fn dynamic_reserved_bytes_offset(token_count: u8) -> usize {
        Self::dynamic_token_vec_offset() + (BORSH_VEC_SIZE_BYTES) 
            + (BORSH_VEC_SIZE_BYTES + size_of::<TokenPosition>() * usize::from(token_count))
            + BORSH_VEC_PADDING_BYTES
    }

    pub fn dynamic_size(token_count: u8) -> usize {
        Self::dynamic_reserved_bytes_offset(token_count) + DYNAMIC_RESERVED_BYTES
    }
}

#[zero_copy]
pub struct PeachAccountFixed {
    pub market: Pubkey,
    pub owner: Pubkey,
    pub name: [u8; 32],
    pub delegate: Pubkey,
    pub account_num: u32,
    being_liquidated: u8,
    in_health_region: u8,
    pub bump: u8,
    pub sequence_number: u8,
    pub net_deposits: i64,
    pub health_region_begin_init_health: i64,
    pub frozen_until: u64,
    pub last_collateral_fee_charge: u64,
}
const_assert_eq!(size_of::<PeachAccountFixed>() % 8, 0);

impl PeachAccountFixed {
    pub fn name(&self) -> &str {
        std::str::from_utf8(&self.name)
            .unwrap()
            .trim_matches(char::from(0))
    }

    pub fn is_operational(&self) -> bool {
        let now_ts: u64 = Clock::get().unwrap().unix_timestamp.try_into().unwrap();
        self.frozen_until < now_ts
    }

    pub fn being_liquidated(&self) -> bool {
        self.being_liquidated == 1
    }

    pub fn set_being_liquidated(&mut self, b: bool) {
        self.being_liquidated = u8::from(b);
    }

    pub fn is_in_health_region(&self) -> bool {
        self.in_health_region == 1
    }

    pub fn set_in_health_region(&mut self, b: bool) {
        self.in_health_region = u8::from(b);
    }

    pub fn maybe_recover_from_being_liquidated(&mut self, liq_end_health: I80F48) -> bool {
        // This is used as threshold to flip flag instead of 0 because of dust issues
        let one_native_usdc = I80F48::ONE;
        if self.being_liquidated() && liq_end_health > -one_native_usdc {
            self.set_being_liquidated(false);
            true
        } else {
            false
        }
    }

    pub fn pda_seeds(&self) -> PeachAccountPdaSeeds {
        PeachAccountPdaSeeds {
            market: self.market,
            owner: self.owner,
            account_num_bytes: self.account_num.to_le_bytes(),
            bump_bytes: [self.bump],
        }
    }

    // pub fn update_protocol_assignment(
    //     &mut self,
    //     mint: &Pubkey,
    //     protocol: u8,
    // ) -> Result<()> {
    //     msg!("Updating protocol assignment for deposit");

    //     let lending_account = &mut self.market;

    //     if let Some(deposit_info) = lending_account.deposits.iter_mut()
    //         .find(|deposit| deposit.mint_key == *mint && deposit.active)
    //     {
    //         msg!("DepositInfo found, updating protocol assignment");
    //         deposit_info.protocol_assign = protocol;
    //     } else {
    //         msg!("DepositInfo not found, cannot update protocol assignment");
    //         return Err(PeachError::DepositNotFound.into());
    //     }

    //     Ok(())
    // }

}

impl Owner for PeachAccountFixed {
    fn owner() -> Pubkey {
        PeachAccount::owner()
    }
}

impl Discriminator for PeachAccountFixed {
    const DISCRIMINATOR: &'static [u8] = PeachAccount::DISCRIMINATOR;
}

impl anchor_lang::ZeroCopy for PeachAccountFixed {}

#[derive(Clone, Debug)]
pub struct PeachAccountDynamicHeader {
    pub token_count: u8,
    // pub serum3_count: u8,
    // pub perp_count: u8,
    // pub perp_oo_count: u8,
    // pub token_conditional_swap_count: u8,
}

impl DynamicHeader for PeachAccountDynamicHeader{
    fn from_bytes(dynamic_data: &[u8]) -> Result<Self> {
        let header_version = u8::from_le_bytes(*array_ref![dynamic_data, 0, size_of::<u8>()]);

        match header_version {
            1 => {
                let token_count = u8::try_from(BorshVecLength::from_le_bytes(*array_ref![
                    dynamic_data,
                    PeachAccount::dynamic_token_vec_offset(),
                    BORSH_VEC_SIZE_BYTES
                ]))
                .unwrap();

                Ok(Self {
                    token_count,
                    // serum3_count: 0,
                    // perp_count: 0,  
                    // perp_oo_count: 0,
                    // token_conditional_swap_count: 0
                })
            }
            _ => err!(PeachError::NotImplementedError).context("unexpected header version number"),
        }
    }

    fn initialize(dynamic_data: &mut [u8]) -> Result<()> {
        let dst: &mut [u8] = &mut dynamic_data[0..1];
        dst.copy_from_slice(&DEFAULT_PEACH_ACCOUNT_VERSION.to_le_bytes());
        Ok(())
    }
}

fn get_helper<T: bytemuck::Pod>(data: &[u8], index: usize) -> &T {
    bytemuck::from_bytes(&data[index..index + size_of::<T>()])
}

fn get_helper_mut<T: bytemuck::Pod>(data: &mut [u8], index: usize) -> &mut T {
    bytemuck::from_bytes_mut(&mut data[index..index + size_of::<T>()])
}

impl PeachAccountDynamicHeader {

    pub fn account_size(&self) -> usize {
        PeachAccount::space(
            self.token_count,
        //     self.serum3_count,
        //     self.perp_count,
        //     self.perp_oo_count,
        //     self.token_conditional_swap_count,
        )
    }

    // offset into dynamic data where 1st TokenPosition would be found
    // todo make fn private
    pub fn token_offset(&self, raw_index: usize) -> usize {
        PeachAccount::dynamic_token_vec_offset()
            + BORSH_VEC_SIZE_BYTES
            + raw_index * size_of::<TokenPosition>()
    }

    fn reserved_bytes_offset(&self) -> usize {
        PeachAccount::dynamic_reserved_bytes_offset(self.token_count)
    }
    
    pub fn token_count(&self) -> usize {
        self.token_count.into()
    }

    pub fn zero() -> Self {
        Self {
            token_count: 0,
            // serum3_count: 0,
            // perp_count: 0,
            // perp_oo_count: 0,
            // token_conditional_swap_count: 0,
        }
    }

    pub fn expected_health_accounts(&self) -> usize {
        self.token_count() * 2 //  + self.serum3_count() + self.perp_count() * 2
    }

    pub fn max_health_accounts() -> usize {
        28
    }

    /// Error if this header isn't a valid resize from `prev`
    ///
    /// - Check that the total health accounts stay limited
    ///   (this coverers token, perp, serum position limits)
    /// - Check that if perp oo/tcs size increases, it is bounded by the limits
    /// - If a field doesn't change, don't error if it exceeds the limits
    ///   (might have been expanded earlier when it was valid to do)
    pub fn check_resize_from(&self, prev: &Self) -> Result<()> {
        let new_health_accounts = self.expected_health_accounts();
        let prev_health_accounts = prev.expected_health_accounts();
        if new_health_accounts > prev_health_accounts {
            require_gte!(Self::max_health_accounts(), new_health_accounts);
        }

        // if self.perp_oo_count > prev.perp_oo_count {
        //     require_gte!(64, self.perp_oo_count);
        // }

        // if self.token_conditional_swap_count > prev.token_conditional_swap_count {
        //     require_gte!(64, self.token_conditional_swap_count);
        // }

        Ok(())
    }
}

/// Fully owned PeachAccount, useful for tests
// pub type PeachAccountValue = DynamicAccount<PeachAccountDynamicHeader, PeachAccountFixed, Vec<u8>>;

/// Full reference type, useful for borrows
pub type PeachAccountRef<'a> =
    DynamicAccount<&'a PeachAccountDynamicHeader, &'a PeachAccountFixed, &'a [u8]>;
/// Full reference type, useful for borrows
pub type PeachAccountRefMut<'a> =
    DynamicAccount<&'a mut PeachAccountDynamicHeader, &'a mut PeachAccountFixed, &'a mut [u8]>;

// Useful when loading from bytes
pub type PeachAccountLoadedRef<'a> =
    DynamicAccount<PeachAccountDynamicHeader, &'a PeachAccountFixed, &'a [u8]>;
/// Useful when loading from RefCell, like from AccountInfo
pub type PeachAccountLoadedRefCell<'a> =
    DynamicAccount<PeachAccountDynamicHeader, Ref<'a, PeachAccountFixed>, Ref<'a, [u8]>>;
/// Useful when loading from RefCell, like from AccountInfo
pub type PeachAccountLoadedRefCellMut<'a> =
    DynamicAccount<PeachAccountDynamicHeader, RefMut<'a, PeachAccountFixed>, RefMut<'a, [u8]>>;

// impl PeachAccountValue {
//     // bytes without discriminator
//     pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
//         let (fixed, dynamic) = bytes.split_at(size_of::<PeachAccountFixed>());
//         Ok(Self {
//             fixed: *bytemuck::from_bytes(fixed),
//             header: PeachAccountDynamicHeader::from_bytes(dynamic)?,
//             dynamic: dynamic.to_vec(),
//         })
//     }
// }

impl<'a> PeachAccountLoadedRef<'a> {
    // bytes without discriminator
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        let (fixed, dynamic) = bytes.split_at(size_of::<PeachAccountFixed>());
        Ok(Self {
            fixed: bytemuck::from_bytes(fixed),
            header: PeachAccountDynamicHeader::from_bytes(dynamic)?,
            dynamic,
        })
    }
}

impl<
        Header: DerefOrBorrow<PeachAccountDynamicHeader>,
        Fixed: DerefOrBorrow<PeachAccountFixed>,
        Dynamic: DerefOrBorrow<[u8]>,
    > DynamicAccount<Header, Fixed, Dynamic>
{
    fn header(&self) -> &PeachAccountDynamicHeader {
        self.header.deref_or_borrow()
    }

    pub fn header_version(&self) -> &u8 {
        get_helper(self.dynamic(), 0)
    }

    fn fixed(&self) -> &PeachAccountFixed {
        self.fixed.deref_or_borrow()
    }

    fn dynamic(&self) -> &[u8] {
        self.dynamic.deref_or_borrow()
    }

    /// Returns
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw/deactivate)
    pub fn token_position_and_raw_index(
        &self,
        token_index: TokenIndex,
    ) -> Result<(&TokenPosition, usize)> {
        self.all_token_positions()
            .enumerate()
            .find_map(|(raw_index, p)| p.is_active_for_token(token_index).then_some((p, raw_index)))
            .ok_or_else(|| {
                error_msg_typed!(
                    PeachError::TokenPositionDoesNotExist,
                    "position for token index {} not found",
                    token_index
                )
            })
    }

    pub fn token_position(&self, token_index: TokenIndex) -> Result<&TokenPosition> {
        self.token_position_and_raw_index(token_index)
            .map(|(p, _)| p)
    }

    pub(crate) fn token_position_by_raw_index_unchecked(&self, raw_index: usize) -> &TokenPosition {
        get_helper(self.dynamic(), self.header().token_offset(raw_index))
    }

    // get iter over all TokenPositions (including inactive)
    pub fn all_token_positions(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        (0..self.header().token_count()).map(|i| self.token_position_by_raw_index_unchecked(i))
    }

    // get iter over all active TokenPositions
    pub fn active_token_positions(&self) -> impl Iterator<Item = &TokenPosition> + '_ {
        self.all_token_positions().filter(|token| token.is_active())
    }

    pub fn being_liquidated(&self) -> bool {
        self.fixed().being_liquidated()
    }

    pub fn borrow(&self) -> PeachAccountRef {
        PeachAccountRef {
            header: self.header(),
            fixed: self.fixed(),
            dynamic: self.dynamic(),
        }
    }
}

impl<
        Header: DerefOrBorrowMut<PeachAccountDynamicHeader> + DerefOrBorrow<PeachAccountDynamicHeader>,
        Fixed: DerefOrBorrowMut<PeachAccountFixed> + DerefOrBorrow<PeachAccountFixed>,
        Dynamic: DerefOrBorrowMut<[u8]> + DerefOrBorrow<[u8]>,
    > DynamicAccount<Header, Fixed, Dynamic>
{
    fn header_mut(&mut self) -> &mut PeachAccountDynamicHeader {
        self.header.deref_or_borrow_mut()
    }

    fn fixed_mut(&mut self) -> &mut PeachAccountFixed {
        self.fixed.deref_or_borrow_mut()
    }

    fn dynamic_mut(&mut self) -> &mut [u8] {
        self.dynamic.deref_or_borrow_mut()
    }

    pub fn borrow_mut(&mut self) -> PeachAccountRefMut {
        PeachAccountRefMut {
            header: self.header.deref_or_borrow_mut(),
            fixed: self.fixed.deref_or_borrow_mut(),
            dynamic: self.dynamic.deref_or_borrow_mut(),
        }
    }

    /// Returns
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw/deactivate)
    pub fn token_position_mut(
        &mut self,
        token_index: TokenIndex,
    ) -> Result<(&mut TokenPosition, usize)> {
        let raw_index = self
            .all_token_positions()
            .enumerate()
            .find_map(|(raw_index, p)| p.is_active_for_token(token_index).then_some(raw_index))
            .ok_or_else(|| {
                error_msg_typed!(
                    PeachError::TokenPositionDoesNotExist,
                    "position for token index {} not found",
                    token_index
                )
            })?;
        Ok((self.token_position_mut_by_raw_index(raw_index), raw_index))
    }

    // get mut TokenPosition at raw_index
    pub fn token_position_mut_by_raw_index(&mut self, raw_index: usize) -> &mut TokenPosition {
        let offset = self.header().token_offset(raw_index);
        get_helper_mut(self.dynamic_mut(), offset)
    }

    /// Creates or retrieves a TokenPosition for the token_index.
    /// Returns:
    /// - the position
    /// - the raw index into the token positions list (for use with get_raw)
    /// - the active index, for use with FixedOrderAccountRetriever
    pub fn ensure_token_position(
        &mut self,
        token_index: TokenIndex,
        is_kamino_position: u8,
    ) -> Result<(&mut TokenPosition, usize, usize)> {
        let mut active_index = 0;
        let mut match_or_free = None;
        for (raw_index, position) in self.all_token_positions().enumerate() {
            if position.is_active_for_token(token_index) {
                // Can't return early because of lifetimes
                match_or_free = Some((raw_index, active_index));
                break;
            }
            if position.is_active() {
                active_index += 1;
            } else if match_or_free.is_none() {
                match_or_free = Some((raw_index, active_index));
            }
        }
        if let Some((raw_index, bank_index)) = match_or_free {
            let v = self.token_position_mut_by_raw_index(raw_index);
            if !v.is_active_for_token(token_index) {
                *v = TokenPosition {
                    indexed_position: FixedWrapper::zero(),
                    token_index,
                    is_kamino_position,
                    in_use_count: 0,
                    padding: [0; 3],
                    _internal_padding_align_prev_idx: [0; 8],
                    previous_index: FixedWrapper::zero(),
                    cumulative_deposit_interest: 0.0,
                    cumulative_borrow_interest: 0.0,
                    _struct_padding_for_pod: [0; 16],
                };
            }
            Ok((v, raw_index, bank_index))
        } else {
            err!(PeachError::NoFreeTokenPositionIndex)
                .context(format!("when looking for token index {}", token_index))
        }
    }

    pub fn deactivate_token_position_and_log(
        &mut self,
        raw_index: usize,
        peach_account_pubkey: Pubkey,
    ) {
        let peach_market = self.fixed().market;
        let token_position = self.token_position_mut_by_raw_index(raw_index);
        assert!(token_position.in_use_count == 0);
        emit_stack(DeactivateTokenPositionLog {
            peach_market: peach_market,
            peach_account: peach_account_pubkey,
            token_index: token_position.token_index.0,
            cumulative_deposit_interest: token_position.cumulative_deposit_interest,
            cumulative_borrow_interest: token_position.cumulative_borrow_interest,
        });
        self.token_position_mut_by_raw_index(raw_index).token_index = TokenIndex::MAX;
    }

    fn write_borsh_vec_length_and_padding(&mut self, offset: usize, count: u8) {
        let dst: &mut [u8] =
            &mut self.dynamic_mut()[offset - BORSH_VEC_SIZE_BYTES - BORSH_VEC_PADDING_BYTES
                ..offset - BORSH_VEC_SIZE_BYTES];
        dst.copy_from_slice(&[0u8; BORSH_VEC_PADDING_BYTES]);
        let dst: &mut [u8] = &mut self.dynamic_mut()[offset - BORSH_VEC_SIZE_BYTES..offset];
        dst.copy_from_slice(&BorshVecLength::from(count).to_le_bytes());
    }

    // writes length of tokens vec at appropriate offset so that borsh can infer the vector length
    // length used is that present in the header
    fn write_token_length(&mut self) {
        let offset = self.header().token_offset(0);
        let count = self.header().token_count;
        self.write_borsh_vec_length_and_padding(offset, count)
    }

    pub fn check_health_pre(&mut self, health_cache: &HealthCache) -> Result<I80F48> {
        let pre_init_health = health_cache.health(HealthType::Init);
        msg!("pre_init_health: {}", pre_init_health);
        self.check_health_pre_checks(health_cache, pre_init_health)?;
        Ok(pre_init_health)
    }

    pub fn check_health_pre_checks(
        &mut self,
        health_cache: &HealthCache,
        pre_init_health: I80F48,
    ) -> Result<()> {
        // We can skip computing LiquidationEnd health if Init health > 0, because
        // LiquidationEnd health >= Init health.
        self.fixed_mut()
            .maybe_recover_from_being_liquidated(pre_init_health);
        if self.fixed().being_liquidated() {
            let liq_end_health = health_cache.health(HealthType::LiquidationEnd);
            self.fixed_mut()
                .maybe_recover_from_being_liquidated(liq_end_health);
        }
        require!(
            !self.fixed().being_liquidated(),
            PeachError::BeingLiquidated
        );
        Ok(())
    }

    pub fn check_health_post(
        &mut self,
        health_cache: &HealthCache,
        pre_init_health: I80F48,
    ) -> Result<I80F48> {
        let post_init_health = health_cache.health(HealthType::Init);
        msg!("post_init_health: {}", post_init_health);
        self.check_health_post_checks(pre_init_health, post_init_health)?;
        Ok(post_init_health)
    }

    pub fn check_health_post_checks(
        &mut self,
        pre_init_health: I80F48,
        post_init_health: I80F48,
    ) -> Result<()> {
        // Accounts that have negative init health may only take actions that don't further
        // decrease their health.
        // To avoid issues with rounding, we allow accounts to decrease their health by up to
        // $1e-6. This is safe because the grace amount is way less than the cost of a transaction.
        // And worst case, users can only use this to gradually drive their own account into
        // liquidation.
        // There is an exception for accounts with health between $0 and -$0.001 (-1000 native),
        // because we don't want to allow empty accounts or accounts with extremely tiny deposits
        // to immediately drive themselves into bankruptcy. (accounts with large deposits can also
        // be in this health range, but it's really unlikely)
        let health_does_not_decrease = if post_init_health < -1000 {
            post_init_health.ceil() >= pre_init_health.ceil()
        } else {
            post_init_health >= pre_init_health
        };

        require!(
            post_init_health >= 0 || health_does_not_decrease,
            PeachError::HealthMustBePositiveOrIncrease
        );
        Ok(())
    }

    /// A stricter version of check_health_post_checks() that requires >=0 health, it not getting
    /// worse is not sufficient
    pub fn check_health_post_checks_strict(&mut self, post_init_health: I80F48) -> Result<()> {
        require!(post_init_health >= 0, PeachError::HealthMustBePositive);
        Ok(())
    }

    pub fn resize_dynamic_content(
        &mut self,
        new_token_count: u8,
        // new_serum3_count: u8,
        // new_perp_count: u8,
        // new_perp_oo_count: u8,
        // new_token_conditional_swap_count: u8,
    ) -> Result<()> {
        let new_header = PeachAccountDynamicHeader {
            token_count: new_token_count,
            // serum3_count: new_serum3_count,
            // perp_count: new_perp_count,
            // perp_oo_count: new_perp_oo_count,
            // token_conditional_swap_count: new_token_conditional_swap_count,
        };
        let old_header = self.header().clone();

        new_header.check_resize_from(&old_header)?;

        let dynamic = self.dynamic_mut();

        // Resizing needs to move the existing bytes in `dynamic` around, preserving
        // existing data, possibly creating new entries or removing unused slots.
        //
        // The operation has four steps:
        // - Defrag: Move all active slots to the front. If a user's token slots were
        //       (unused, token pos for 4, unused, token pos for 500, unused)
        //   before, they'd be
        //       (token pos for 4, token pos for 500, garbage, garbage, garbage)
        //   after. That way all data that needs to be preserved for each type of
        //   slot is one contiguous block.
        // - Moving preserved blocks to the left where needed, iterating blocks left to right.
        // - Moving preserved blocks to the right where needed, iterating blocks right to left.
        // - Default-initializing all non-preserved spaces.

        // "Defrag" token, serum, perp by moving active positions into the front slots
        //
        // Dangerous because this does NOT reset the previous positions!
        // Use the active_* values to know how many slots are in-use afterwards!
        //
        // Perp OOs can't be collapsed this way because LeafNode::owner_slot is an index into it.
        let mut active_token_positions = 0;
        for i in 0..old_header.token_count() {
            let src = old_header.token_offset(i);
            let pos: &TokenPosition = get_helper(dynamic, src);
            if !pos.is_active() {
                continue;
            }
            if i != active_token_positions {
                let dst = old_header.token_offset(active_token_positions);
                unsafe {
                    sol_memmove(
                        &mut dynamic[dst],
                        &mut dynamic[src],
                        size_of::<TokenPosition>(),
                    );
                }
            }
            active_token_positions += 1;
        }

        // let mut active_serum3_orders = 0;
        // for i in 0..old_header.serum3_count() {
        //     let src = old_header.serum3_offset(i);
        //     let pos: &Serum3Orders = get_helper(dynamic, src);
        //     if !pos.is_active() {
        //         continue;
        //     }
        //     if i != active_serum3_orders {
        //         let dst = old_header.serum3_offset(active_serum3_orders);
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[dst],
        //                 &mut dynamic[src],
        //                 size_of::<Serum3Orders>(),
        //             );
        //         }
        //     }
        //     active_serum3_orders += 1;
        // }

        // let mut active_perp_positions = 0;
        // for i in 0..old_header.perp_count() {
        //     let src = old_header.perp_offset(i);
        //     let pos: &PerpPosition = get_helper(dynamic, src);
        //     if !pos.is_active() {
        //         continue;
        //     }
        //     if i != active_perp_positions {
        //         let dst = old_header.perp_offset(active_perp_positions);
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[dst],
        //                 &mut dynamic[src],
        //                 size_of::<PerpPosition>(),
        //             );
        //         }
        //     }
        //     active_perp_positions += 1;
        // }

        // Can't rearrange perp oo because LeafNodes store indexes, so the equivalent
        // to the "active" count for the other blocks is the max active index + 1.
        // let mut blocked_perp_oo = 0;
        // for i in 0..old_header.perp_oo_count() {
        //     let idx = old_header.perp_oo_count() - 1 - i;
        //     let src = old_header.perp_oo_offset(idx);
        //     let pos: &PerpOpenOrder = get_helper(dynamic, src);
        //     if pos.is_active() {
        //         blocked_perp_oo = idx + 1;
        //         break;
        //     }
        // }

        // let mut active_tcs = 0;
        // for i in 0..old_header.token_conditional_swap_count() {
        //     let src = old_header.token_conditional_swap_offset(i);
        //     let pos: &TokenConditionalSwap = get_helper(dynamic, src);
        //     if !pos.is_configured() {
        //         continue;
        //     }
        //     if i != active_tcs {
        //         let dst = old_header.token_conditional_swap_offset(active_tcs);
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[dst],
        //                 &mut dynamic[src],
        //                 size_of::<TokenConditionalSwap>(),
        //             );
        //         }
        //     }
        //     active_tcs += 1;
        // }

        // Check that the new allocations can fit the existing data
        require_gte!(new_header.token_count(), active_token_positions);
        // require_gte!(new_header.serum3_count(), active_serum3_orders);
        // require_gte!(new_header.perp_count(), active_perp_positions);
        // require_gte!(new_header.perp_oo_count(), blocked_perp_oo);
        // require_gte!(new_header.token_conditional_swap_count(), active_tcs);

        // First move pass: go left-to-right and move any blocks that need to be moved
        // to the left. This will never overwrite other data, because:
        // - moving to the left can only overwrite data to the left
        // - the left of the target location is >= the right of the previous data location
        //   because either the previous was already moved to the left (clearly good),
        //   or still needs to be moved to the right (the new end will be <= the target start)
        // {
        //     // Token positions never move

        //     let old_serum3_start = old_header.serum3_offset(0);
        //     let new_serum3_start = new_header.serum3_offset(0);
        //     if new_serum3_start < old_serum3_start && active_serum3_orders > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_serum3_start],
        //                 &mut dynamic[old_serum3_start],
        //                 size_of::<Serum3Orders>() * active_serum3_orders,
        //             );
        //         }
        //     }

        //     let old_perp_start = old_header.perp_offset(0);
        //     let new_perp_start = new_header.perp_offset(0);
        //     if new_perp_start < old_perp_start && active_perp_positions > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_perp_start],
        //                 &mut dynamic[old_perp_start],
        //                 size_of::<PerpPosition>() * active_perp_positions,
        //             );
        //         }
        //     }

        //     let old_perp_oo_start = old_header.perp_oo_offset(0);
        //     let new_perp_oo_start = new_header.perp_oo_offset(0);
        //     if new_perp_oo_start < old_perp_oo_start && blocked_perp_oo > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_perp_oo_start],
        //                 &mut dynamic[old_perp_oo_start],
        //                 size_of::<PerpOpenOrder>() * blocked_perp_oo,
        //             );
        //         }
        //     }

        //     let old_tcs_start = old_header.token_conditional_swap_offset(0);
        //     let new_tcs_start = new_header.token_conditional_swap_offset(0);
        //     if new_tcs_start < old_tcs_start && active_tcs > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_tcs_start],
        //                 &mut dynamic[old_tcs_start],
        //                 size_of::<TokenConditionalSwap>() * active_tcs,
        //             );
        //         }
        //     }
        // }

        // Second move pass: Go right-to-left and move everything to the right if needed.
        // This will never overwrite other data:
        // - because of moving right, it could only overwrite a block to the right
        // - if the block to the right needed moving to the right, that was already done
        // - if the block to the right was moved to the left, we know that its start will
        //   be >= our block's end
        // {
        //     let old_tcs_start = old_header.token_conditional_swap_offset(0);
        //     let new_tcs_start = new_header.token_conditional_swap_offset(0);
        //     if new_tcs_start > old_tcs_start && active_tcs > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_tcs_start],
        //                 &mut dynamic[old_tcs_start],
        //                 size_of::<TokenConditionalSwap>() * active_tcs,
        //             );
        //         }
        //     }

        //     let old_perp_oo_start = old_header.perp_oo_offset(0);
        //     let new_perp_oo_start = new_header.perp_oo_offset(0);
        //     if new_perp_oo_start > old_perp_oo_start && blocked_perp_oo > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_perp_oo_start],
        //                 &mut dynamic[old_perp_oo_start],
        //                 size_of::<PerpOpenOrder>() * blocked_perp_oo,
        //             );
        //         }
        //     }

        //     let old_perp_start = old_header.perp_offset(0);
        //     let new_perp_start = new_header.perp_offset(0);
        //     if new_perp_start > old_perp_start && active_perp_positions > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_perp_start],
        //                 &mut dynamic[old_perp_start],
        //                 size_of::<PerpPosition>() * active_perp_positions,
        //             );
        //         }
        //     }

        //     let old_serum3_start = old_header.serum3_offset(0);
        //     let new_serum3_start = new_header.serum3_offset(0);
        //     if new_serum3_start > old_serum3_start && active_serum3_orders > 0 {
        //         unsafe {
        //             sol_memmove(
        //                 &mut dynamic[new_serum3_start],
        //                 &mut dynamic[old_serum3_start],
        //                 size_of::<Serum3Orders>() * active_serum3_orders,
        //             );
        //         }
        //     }

        //     // Token positions never move
        // }

        // Defaulting pass: The blocks are in their final positions, clear out all unused slots
        {
            for i in active_token_positions..new_header.token_count() {
                *get_helper_mut(dynamic, new_header.token_offset(i)) = TokenPosition::default();
            }
            // for i in active_serum3_orders..new_header.serum3_count() {
            //     *get_helper_mut(dynamic, new_header.serum3_offset(i)) = Serum3Orders::default();
            // }
            // for i in active_perp_positions..new_header.perp_count() {
            //     *get_helper_mut(dynamic, new_header.perp_offset(i)) = PerpPosition::default();
            // }
            // for i in blocked_perp_oo..new_header.perp_oo_count() {
            //     *get_helper_mut(dynamic, new_header.perp_oo_offset(i)) = PerpOpenOrder::default();
            // }
            // for i in active_tcs..new_header.token_conditional_swap_count() {
            //     *get_helper_mut(dynamic, new_header.token_conditional_swap_offset(i)) =
            //         TokenConditionalSwap::default();
            // }
        }
        {
            let offset = new_header.reserved_bytes_offset();
            dynamic[offset..offset + DYNAMIC_RESERVED_BYTES]
                .copy_from_slice(&[0u8; DYNAMIC_RESERVED_BYTES]);
        }

        // update the already-parsed header
        *self.header_mut() = new_header;

        // write new lengths to the dynamic data (uses header)
        self.write_token_length();
        // self.write_serum3_length();
        // self.write_perp_length();
        // self.write_perp_oo_length();
        // self.write_token_conditional_swap_length();

        Ok(())
    }
}

/// Trait to allow a AccountLoader<PeachAccountFixed> to create an accessor for the full account.
pub trait PeachAccountLoader<'a> {
    fn load_full(self) -> Result<PeachAccountLoadedRefCell<'a>>;
    fn load_full_mut(self) -> Result<PeachAccountLoadedRefCellMut<'a>>;
    fn load_full_init(self) -> Result<PeachAccountLoadedRefCellMut<'a>>;
}

impl<'a, 'info: 'a> PeachAccountLoader<'a> for &'a AccountLoader<'info, PeachAccountFixed> {
    fn load_full(self) -> Result<PeachAccountLoadedRefCell<'a>> {
        // Error checking
        self.load()?;

        let data = self.as_ref().try_borrow_data()?;
        let header =
            PeachAccountDynamicHeader::from_bytes(&data[8 + size_of::<PeachAccountFixed>()..])?;
        let (_, data) = Ref::map_split(data, |d| d.split_at(8));
        let (fixed_bytes, dynamic) =
            Ref::map_split(data, |d| d.split_at(size_of::<PeachAccountFixed>()));
        Ok(PeachAccountLoadedRefCell {
            header,
            fixed: Ref::map(fixed_bytes, |b| bytemuck::from_bytes(b)),
            dynamic,
        })
    }

    fn load_full_mut(self) -> Result<PeachAccountLoadedRefCellMut<'a>> {
        // Error checking
        self.load_mut()?;

        let data = self.as_ref().try_borrow_mut_data()?;
        let header =
            PeachAccountDynamicHeader::from_bytes(&data[8 + size_of::<PeachAccountFixed>()..])?;
        let (_, data) = RefMut::map_split(data, |d| d.split_at_mut(8));
        let (fixed_bytes, dynamic) =
            RefMut::map_split(data, |d| d.split_at_mut(size_of::<PeachAccountFixed>()));
        Ok(PeachAccountLoadedRefCellMut {
            header,
            fixed: RefMut::map(fixed_bytes, |b| bytemuck::from_bytes_mut(b)),
            dynamic,
        })
    }

    fn load_full_init(self) -> Result<PeachAccountLoadedRefCellMut<'a>> {
        // Error checking
        self.load_init()?;

        {
            let mut data = self.as_ref().try_borrow_mut_data()?;

            let disc_bytes: &mut [u8] = &mut data[0..8];
            // disc_bytes.copy_from_slice(bytemuck::bytes_of(PeachAccount::DISCRIMINATOR));
            disc_bytes.copy_from_slice(&PeachAccount::DISCRIMINATOR);
            PeachAccountDynamicHeader::initialize(&mut data[8 + size_of::<PeachAccountFixed>()..])?;
        }

        self.load_full_mut()
    }
}