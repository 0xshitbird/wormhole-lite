use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::AccountMeta,
    log::sol_log,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::state::emitter::Emitter;

pub struct TransactionAccountKeys {
    /// account used to pay for fees
    pub payer: Pubkey,
    /// the emitter account
    pub emitter: Pubkey,
    /// system program
    pub system_program: Pubkey,
}

impl TransactionAccountKeys {
    /// returns a vector of AccountMeta objects for sending a tx from an rpc client
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.payer, true),
            AccountMeta::new(self.emitter, false),
            AccountMeta::new_readonly(self.system_program, false),
        ]
    }
}
/// onchian object ponting to actual accounts
pub struct InitializeEmitterAccounts<'info> {
    pub payer: AccountInfo<'info>,
    pub emitter: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

impl<'info> From<&[AccountInfo<'info>]> for InitializeEmitterAccounts<'info> {
    fn from(value: &[AccountInfo<'info>]) -> Self {
        Self {
            payer: value.get(0).unwrap().clone(),
            emitter: value.get(1).unwrap().clone(),
            system_program: value.get(2).unwrap().clone(),
        }
    }
}

impl<'info> From<InitializeEmitterAccounts<'info>> for TransactionAccountKeys {
    fn from(value: InitializeEmitterAccounts<'info>) -> Self {
        Self {
            payer: *value.payer.key,
            emitter: *value.emitter.key,
            system_program: *value.system_program.key,
        }
    }
}

impl<'info> InitializeEmitterAccounts<'info> {
    pub fn validate(&self, expected_pda: Pubkey) -> bool {
        if self.emitter.key.ne(&expected_pda) {
            sol_log("invalid emitter");
            return false;
        }
        if self.system_program.key.ne(&system_program::id()) {
            sol_log("invalid system program");
            return false;
        }
        return true;
    }
    pub fn try_validate(&self, expected_pda: Pubkey) {
        if !self.validate(expected_pda) {
            panic!("validation failed");
        }
    }
}

pub fn initialize_emitter<'info>(
    program_id: Pubkey,
    accounts: &[AccountInfo<'info>],
) -> ProgramResult {
    let account_infos = InitializeEmitterAccounts::from(accounts);

    let (emitter_pda, emitter_nonce) = crate::utils::derivations::derive_emitter(program_id);

    account_infos.try_validate(emitter_pda);

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(Emitter::LEN);

    invoke_signed(
        &system_instruction::create_account(
            account_infos.payer.key,
            account_infos.emitter.key,
            lamports,
            Emitter::LEN as u64,
            &program_id,
        ),
        &[account_infos.payer.clone(), account_infos.emitter.clone()],
        &[&[Emitter::seed(), &[emitter_nonce]]],
    )?;

    let mut account = Emitter::unpack_unchecked(&account_infos.emitter.data.borrow())?;
    if account.is_initialized() {
        sol_log("account already in use");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if !rent.is_exempt(
        account_infos.emitter.lamports(),
        account_infos.emitter.data_len(),
    ) {
        sol_log("account not rent exempt");
        return Err(ProgramError::AccountNotRentExempt);
    }
    account.owner = program_id;
    account.nonce = emitter_nonce;
    Emitter::pack(account, &mut account_infos.emitter.data.borrow_mut())?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::utils::derivations::derive_emitter;

    use super::*;
    #[test]
    fn test_transaction_account_keys() {
        let payer = Pubkey::new_unique();
        let accts = TransactionAccountKeys {
            payer,
            emitter: derive_emitter(system_program::id()).0,
            system_program: system_program::id(),
        };
        let acct_metas = accts.to_account_metas();
        assert_eq!(
            acct_metas,
            vec![
                AccountMeta::new(accts.payer, true),
                AccountMeta::new(accts.emitter, false),
                AccountMeta::new_readonly(accts.system_program, false),
            ]
        );
    }
    #[test]
    fn test_account_infos() {
        let mut data = vec![5; 80];
        let mut lamports = 42;
        let mut data2 = vec![5; 80];
        let mut lamports2 = 42;
        let mut data3 = vec![5; 80];
        let mut lamports3 = 42;
        let pid = Pubkey::new_unique();
        let sys_id = system_program::id();
        let payer = Pubkey::new_unique();
        let emitter_pda = derive_emitter(pid).0;
        let accts = TransactionAccountKeys {
            payer,
            emitter: emitter_pda,
            system_program: system_program::id(),
        };
        let emitter = AccountInfo::new(
            &accts.emitter,
            false,
            false,
            &mut lamports,
            &mut data,
            &pid,
            false,
            0,
        );
        let payer = AccountInfo::new(
            &accts.payer,
            false,
            false,
            &mut lamports2,
            &mut data2,
            &sys_id,
            false,
            0,
        );
        let system_program = AccountInfo::new(
            &accts.system_program,
            false,
            false,
            &mut lamports3,
            &mut data3,
            &sys_id,
            false,
            0,
        );
        let account_infos = vec![payer, emitter, system_program];
        let emitter_accounts = InitializeEmitterAccounts::from(&account_infos[..]);
        assert!(emitter_accounts.validate(emitter_pda));
        assert!(!emitter_accounts.validate(system_program::id()));
    }
}
