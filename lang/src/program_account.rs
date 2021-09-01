use crate::error::ErrorCode;
use crate::{
    AccountDeserialize, AccountSerialize, Accounts, AccountsClose, AccountsExit, CpiAccount, Key,
    ToAccountInfo, ToAccountInfos, ToAccountMetas,
};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

/// Boxed container for a deserialized `account`. Use this to reference any
/// account owned by the currently executing program.
#[derive(Clone)]
pub struct ProgramAccount<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    inner: Box<Inner<'info, T>>,
}

#[derive(Clone)]
struct Inner<'info, T: AccountSerialize + AccountDeserialize + Clone> {
    info: AccountInfo<'info>,
    account: T,
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> ProgramAccount<'a, T> {
    pub fn new(info: AccountInfo<'a>, account: T) -> ProgramAccount<'a, T> {
        Self {
            inner: Box::new(Inner { info, account }),
        }
    }

    /// Deserializes the given `info` into a `ProgramAccount`.
    #[inline(never)]
    pub fn try_from(program_id: &Pubkey, info: &AccountInfo<'a>) -> Result<ProgramAccount<'a, T>, ProgramError> {
        if info.owner != program_id {
            return Err(ErrorCode::AccountNotProgramOwned.into());
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize(&mut data)?,
        ))
    }


		/// Deserializes the given `info` into a `ProgramAccount` without checking
		/// the account discriminator. Be careful when using this and avoid it if
		/// possible.
    #[inline(never)]
    pub fn try_from_unchecked(
				program_id: &Pubkey,
        info: &AccountInfo<'a>,
    ) -> Result<ProgramAccount<'a, T>, ProgramError> {
        if info.owner != program_id {
            return Err(ErrorCode::AccountNotProgramOwned.into());
        }
        let mut data: &[u8] = &info.try_borrow_data()?;
        Ok(ProgramAccount::new(
            info.clone(),
            T::try_deserialize_unchecked(&mut data)?,
        ))
    }

    pub fn into_inner(self) -> T {
        self.inner.account
    }
}

impl<'info, T> Accounts<'info> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    #[inline(never)]
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &mut &[AccountInfo<'info>],
        _ix_data: &[u8],
    ) -> Result<Self, ProgramError> {
        if accounts.is_empty() {
            return Err(ErrorCode::AccountNotEnoughKeys.into());
        }
        let account = &accounts[0];
        *accounts = &accounts[1..];
				ProgramAccount::try_from(program_id, account)
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsExit<'info>
    for ProgramAccount<'info, T>
{
    fn exit(&self, _program_id: &Pubkey) -> ProgramResult {
        let info = self.to_account_info();
        let mut data = info.try_borrow_mut_data()?;
        let dst: &mut [u8] = &mut data;
        let mut cursor = std::io::Cursor::new(dst);
        self.inner.account.try_serialize(&mut cursor)?;
        Ok(())
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AccountsClose<'info>
    for ProgramAccount<'info, T>
{
    fn close(&self, sol_destination: AccountInfo<'info>) -> ProgramResult {
        crate::common::close(self.to_account_info(), sol_destination)
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountMetas
    for ProgramAccount<'info, T>
{
    fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
        let is_signer = is_signer.unwrap_or(self.inner.info.is_signer);
        let meta = match self.inner.info.is_writable {
            false => AccountMeta::new_readonly(*self.inner.info.key, is_signer),
            true => AccountMeta::new(*self.inner.info.key, is_signer),
        };
        vec![meta]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfos<'info>
    for ProgramAccount<'info, T>
{
    fn to_account_infos(&self) -> Vec<AccountInfo<'info>> {
        vec![self.inner.info.clone()]
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> ToAccountInfo<'info>
    for ProgramAccount<'info, T>
{
    fn to_account_info(&self) -> AccountInfo<'info> {
        self.inner.info.clone()
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> AsRef<AccountInfo<'info>>
    for ProgramAccount<'info, T>
{
    fn as_ref(&self) -> &AccountInfo<'info> {
        &self.inner.info
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> Deref for ProgramAccount<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &(*self.inner).account
    }
}

impl<'a, T: AccountSerialize + AccountDeserialize + Clone> DerefMut for ProgramAccount<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut DerefMut::deref_mut(&mut self.inner).account
    }
}

impl<'info, T> From<CpiAccount<'info, T>> for ProgramAccount<'info, T>
where
    T: AccountSerialize + AccountDeserialize + Clone,
{
    fn from(a: CpiAccount<'info, T>) -> Self {
        Self::new(a.to_account_info(), Deref::deref(&a).clone())
    }
}

impl<'info, T: AccountSerialize + AccountDeserialize + Clone> Key for ProgramAccount<'info, T> {
    fn key(&self) -> Pubkey {
        *self.inner.info.key
    }
}
