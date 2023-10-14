use borsh::ser::BorshSerialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction, sysvar, system_program,
};
use wormhole_anchor_sdk::wormhole::Finality;
use solana_program::log::sol_log;
use crate::{state::emitter::Emitter, utils::derivations::derive_message_pda, WORMHOLE_PROGRAM_ID};
/// when invoking an instruction that publishes a message through wormhole, these are the accounts
/// that must be used in the instruction
pub struct TransactionAccountKeys {
    /// account used to pay for fees
    pub payer: Pubkey,
    /// account used for handling message emittion
    /// seed: [b"emitter"]
    pub emitter: Pubkey,
    /// core bridge program account
    /// seed: [b"Bridge"]
    pub core_bridge_config: Pubkey,
    /// core bridge program sequence tracking account
    /// seed: [b"Sequence", PROGRAM_ID]
    pub core_emitter_sequence: Pubkey,
    /// core bridge program message contents account
    /// may be a keypair or pda controlled by our program
    pub core_message_account: Pubkey,
    /// main wormhole program
    pub core_bridge_program: Pubkey,
    /// core bridge program fee collector
    pub core_fee_collector: Pubkey,
    /// system program
    pub system_program: Pubkey,
    /// clock sysvar
    pub clock: Pubkey,
    /// rent sysvar
    pub rent: Pubkey,
}

impl TransactionAccountKeys {
    /// returns a vector of AccountMeta objects for sending a tx from an rpc client
    pub fn to_account_metas(&self) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new(self.core_bridge_config, false), // 0
            AccountMeta::new(self.core_message_account, false), // 1
            AccountMeta::new(self.emitter, false),            // 2
            AccountMeta::new(self.core_emitter_sequence, false), // 3
            AccountMeta::new(self.payer, true),               // 4
            AccountMeta::new(self.core_fee_collector, false), // 5
            AccountMeta::new_readonly(self.clock, false),     // 6
            AccountMeta::new_readonly(self.system_program, false), // 7
            AccountMeta::new_readonly(self.rent, false),      // 8
            AccountMeta::new_readonly(self.core_bridge_program, false), // 9
        ]
    }
}

/// on-chain object pointing to the actual accounts
pub struct Accounts<'info> {
    /// account used to pay for fees
    pub payer: AccountInfo<'info>,
    /// account used for handling message emittion
    /// seed: [b"emitter"]
    pub emitter: AccountInfo<'info>,
    /// core bridge program account
    /// seed: [b"Bridge"]
    pub core_bridge_config: AccountInfo<'info>,
    /// core bridge program sequence tracking account
    /// seed: [b"Sequence", PROGRAM_ID]
    pub core_emitter_sequence: AccountInfo<'info>,
    /// core bridge program message contents account
    /// may be a keypair or pda controlled by our program
    pub core_message_account: AccountInfo<'info>,
    /// main wormhole program
    pub core_bridge_program: AccountInfo<'info>,
    /// core bridge program fee collector
    pub core_fee_collector: AccountInfo<'info>,
    /// system program
    pub system_program: AccountInfo<'info>,
    /// clock sysvar account
    pub clock: AccountInfo<'info>,
    /// rent sysvar account
    pub rent: AccountInfo<'info>,
}

impl<'info> From<&[AccountInfo<'info>]> for Accounts<'info> {
    fn from(value: &[AccountInfo<'info>]) -> Self {
        Self {
            core_bridge_config: value.get(0).unwrap().clone(),
            core_message_account: value.get(1).unwrap().clone(),
            emitter: value.get(2).unwrap().clone(),
            core_emitter_sequence: value.get(3).unwrap().clone(),
            payer: value.get(4).unwrap().clone(),
            core_fee_collector: value.get(5).unwrap().clone(),
            clock: value.get(6).unwrap().clone(),
            system_program: value.get(7).unwrap().clone(),
            rent: value.get(8).unwrap().clone(),
            core_bridge_program: value.get(9).unwrap().clone(), // last account in the slice
        }
    }
}

impl<'info> From<&Accounts<'info>> for TransactionAccountKeys {
    fn from(value: &Accounts<'info>) -> Self {
        TransactionAccountKeys {
            payer: *value.payer.key,
            emitter: *value.emitter.key,
            core_bridge_config: *value.core_bridge_config.key,
            core_emitter_sequence: *value.core_emitter_sequence.key,
            core_message_account: *value.core_message_account.key,
            core_bridge_program: *value.core_bridge_program.key,
            core_fee_collector: *value.core_fee_collector.key,
            system_program: *value.system_program.key,
            clock: *value.clock.key,
            rent: *value.rent.key,
        }
    }
}

impl<'info> Accounts<'info> {
    /// converts the Accounts object into a vector of AccountInfos, used for cpi
    pub fn to_vec(&self) -> Vec<AccountInfo<'info>> {
        vec![
            self.core_bridge_config.clone(),
            self.core_message_account.clone(),
            self.emitter.clone(),
            self.core_emitter_sequence.clone(),
            self.payer.clone(),
            self.core_fee_collector.clone(),
            self.clock.clone(),
            self.system_program.clone(),
            self.rent.clone(),
        ]
    }
    /// creates an instruction which is used to seed the fee collector with fees
    ///
    /// must be invoked first
    pub fn fee_collector_ix(&self) -> Instruction {
        system_instruction::transfer(self.payer.key, self.core_fee_collector.key, 100)
    }
    /// creates an instruction which is used to post a message to wormhole
    pub fn post_message_ix(
        &self,
        batch_id: u32,
        payload: Vec<u8>,
        finality: Finality,
    ) -> Instruction {
        Instruction {
            program_id: *self.core_bridge_program.key,
            accounts: TransactionAccountKeys::from(self).to_account_metas(),
            data: wormhole_anchor_sdk::wormhole::Instruction::PostMessage {
                batch_id,
                payload,
                finality,
            }
            .try_to_vec()
            .unwrap(),
        }
    }
    /// validates the account information, returning true if verification passes
    pub fn validate(
        &self,
        emitter_pda: Pubkey,
        message_pda: Pubkey,
        executing_program_id: Pubkey,
    ) -> bool {
        // validate account keys
        if self.clock.key.ne(&sysvar::clock::id()) {
            sol_log("invalid clock");
            return false;
        }
        if self.rent.key.ne(&sysvar::rent::id()) {
            sol_log("invalid rent");
            return false;
        }
        if self.system_program.key.ne(&system_program::id()) {
            sol_log("invalid system program");
            return false;
        }
        if self.core_bridge_program.key.ne(&WORMHOLE_PROGRAM_ID) {
            sol_log("invalid core bridge program");
            return false;
        }
        if self.emitter.key.ne(&emitter_pda) {
            sol_log("invalid emitter");
            return false;
        }
        if self.core_message_account.key.ne(&message_pda) {
            sol_log("invalid message");
            return false;
        }
        // validate account owners
        if executing_program_id.ne(self.emitter.owner) {
            sol_log("invalid emitter account owner");
            return false;
        }
        if self.core_bridge_config.key.ne(self.core_bridge_program.key) {
            sol_log("invalid bridge config owner");
            return false;
        }
        if self.core_fee_collector.key.ne(self.core_bridge_program.key) {
            sol_log("invalid fee collector owner");
            return false;
        }
        // other ownership doesnt need to be verified since that is handle by wormhole program
        true
    }
    pub fn try_validate(&self, emitter_pda: Pubkey, message_pda: Pubkey, executing_program_id: Pubkey) {
        if !self.validate(emitter_pda, message_pda, executing_program_id) {
            panic!("invalid accounts");
        }
    }
    /// sends a message on wormhole
    /// https://docs.rs/wormhole-core-bridge-solana/0.0.0-alpha.6/wormhole_core_bridge_solana/
    pub fn send_message(
        &self,
        // address of the program invoking teh cpi call
        executing_program_id: Pubkey,
        batch_id: u32,
        payload: Vec<u8>,
    ) -> ProgramResult {

        let next_publishable_nonce =
            Emitter::slice_next_publishable_nonce(&self.emitter.data.borrow());
        let (emitter_pda, emitter_nonce) = Emitter::derive(executing_program_id);
        let (message_pda, message_nonce) =
            derive_message_pda(executing_program_id, next_publishable_nonce);

        // validate all accounts to be used in the instruction
        self.try_validate(emitter_pda, message_pda, executing_program_id);

        let ix = self.fee_collector_ix();
        invoke(&ix, &[self.payer.clone(), self.core_fee_collector.clone()])?;


        let ix = self.post_message_ix(batch_id, payload, Finality::Finalized);
        invoke_signed(
            &ix,
            &self.to_vec(),
            &[
                &[Emitter::seed(), &[emitter_nonce]],
                &[
                    b"message",
                    &next_publishable_nonce.to_le_bytes()[..],
                    &[message_nonce],
                ],
            ],
        )?;

        // increment the nonce used for message account derivation
        let mut emitter = Emitter::unpack(&self.emitter.data.borrow())?;
        emitter.next_publishable_nonce = emitter.next_publishable_nonce.checked_add(1).unwrap();
        Emitter::pack(emitter, &mut self.emitter.data.borrow_mut())?;
        Ok(())
    }
}
