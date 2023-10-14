use crate::{state::emitter::Emitter, utils::derivations::derive_message_pda, WORMHOLE_PROGRAM_ID};
use borsh::ser::BorshSerialize;
use solana_program::log::sol_log;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction, system_program, sysvar,
};
use wormhole_anchor_sdk::wormhole::Finality;
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
        sequence_pda: Pubkey,
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
        if self.core_emitter_sequence.key.ne(&sequence_pda) {
            sol_log("invalid sequence");
            return false;
        }
        // validate account owners
        if executing_program_id.ne(self.emitter.owner) {
            sol_log("invalid emitter account owner");
            return false;
        }
        if self
            .core_bridge_config
            .owner
            .ne(self.core_bridge_program.key)
        {
            sol_log("invalid bridge config owner");
            return false;
        }
        if self
            .core_fee_collector
            .owner
            .ne(self.core_bridge_program.key)
        {
            sol_log("invalid fee collector owner");
            return false;
        }
        if self.emitter.owner.ne(&executing_program_id) {
            sol_log("invalid emitter owner");
            return false;
        }
        // sequence account may not be initialized yet
        // other ownership doesnt need to be verified since that is handle by wormhole program
        true
    }
    pub fn try_validate(
        &self,
        emitter_pda: Pubkey,
        message_pda: Pubkey,
        sequence_pda: Pubkey,
        executing_program_id: Pubkey,
    ) {
        if !self.validate(emitter_pda, message_pda, sequence_pda, executing_program_id) {
            panic!("invalid accounts");
        }
    }
    /// sends a message via wormhole using CPI
    /// https://docs.rs/wormhole-core-bridge-solana/0.0.0-alpha.6/wormhole_core_bridge_solana/
    ///
    /// this is not tested within this actual crate
    pub fn send_message(
        &self,
        // address of the program invoking teh cpi call
        executing_program_id: Pubkey,
        batch_id: u32,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let (sequence_pda, _, emitter_pda, emitter_nonce) = {
            let emitter = Emitter::unpack(&self.emitter.data.borrow())?;
            let (sequence_pda, sequence_nonce) = emitter.derive_sequence();
            let (emitter_pda, emitter_nonce) = emitter.derive();
            (sequence_pda, sequence_nonce, emitter_pda, emitter_nonce)
        };
        let next_publishable_nonce =
            Emitter::slice_next_publishable_nonce(&self.emitter.data.borrow());
        let (message_pda, message_nonce) =
            derive_message_pda(executing_program_id, next_publishable_nonce);

        // validate all accounts to be used in the instruction
        self.try_validate(emitter_pda, message_pda, sequence_pda, executing_program_id);

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

#[cfg(test)]
mod test {
    use solana_program::system_instruction::SystemInstruction;

    use crate::{
        utils::derivations::{
            derive_core_bridge_config, derive_core_fee_collector, derive_emitter, derive_sequence,
        },
        WORMHOLE_TOKEN_BRIDGE_PROGRAM_ID,
    };

    use super::*;
    fn core_bridge_config() -> Pubkey {
        derive_core_bridge_config().0
    }
    fn core_message_account(program_id: Pubkey, nonce: u64) -> Pubkey {
        derive_message_pda(program_id, nonce).0
    }
    fn emitter(program_id: Pubkey) -> Pubkey {
        derive_emitter(program_id).0
    }
    fn core_emitter_sequence(emitter: Pubkey) -> Pubkey {
        derive_sequence(emitter).0
    }
    fn payer() -> Pubkey {
        let mut info: [u8; 32] = [0_u8; 32];
        info[0] = 5;
        info[1] = 5;
        Pubkey::new_from_array(info)
    }
    fn core_fee_collector() -> Pubkey {
        derive_core_fee_collector().0
    }
    #[test]
    fn test_transaction_account_keys() {
        let pid = WORMHOLE_TOKEN_BRIDGE_PROGRAM_ID;
        let accts = TransactionAccountKeys {
            core_bridge_config: core_bridge_config(),
            core_message_account: core_message_account(pid, 69),
            emitter: emitter(pid),
            core_emitter_sequence: core_emitter_sequence(emitter(pid)),
            payer: payer(),
            core_fee_collector: core_fee_collector(),
            clock: sysvar::clock::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
            core_bridge_program: WORMHOLE_PROGRAM_ID,
        };
        let expected_metas = vec![
            AccountMeta::new(accts.core_bridge_config, false), // 0
            AccountMeta::new(accts.core_message_account, false), // 1
            AccountMeta::new(accts.emitter, false),            // 2
            AccountMeta::new(accts.core_emitter_sequence, false), // 3
            AccountMeta::new(accts.payer, true),               // 4
            AccountMeta::new(accts.core_fee_collector, false), // 5
            AccountMeta::new_readonly(accts.clock, false),     // 6
            AccountMeta::new_readonly(accts.system_program, false), // 7
            AccountMeta::new_readonly(accts.rent, false),      // 8
            AccountMeta::new_readonly(accts.core_bridge_program, false), // 9
        ];
        let got_metas = accts.to_account_metas();
        assert_eq!(got_metas, expected_metas);
    }
    #[test]
    fn test_account_infos() {
        let key = Pubkey::new_unique();
        let mut data = vec![5; 80];
        let mut lamports = 42;
        let mut data2 = vec![5; 80];
        let mut lamports2 = 42;
        let mut data3 = vec![5; 80];
        let mut lamports3 = 42;
        let mut data4 = vec![5; 80];
        let mut lamports4 = 42;
        let mut data5 = vec![5; 80];
        let mut lamports5 = 42;
        let mut data6 = vec![5; 80];
        let mut lamports6 = 42;
        let mut data7 = vec![5; 80];
        let mut lamports7 = 42;
        let mut data8 = vec![5; 80];
        let mut lamports8 = 42;
        let mut data9 = vec![5; 80];
        let mut lamports9 = 42;
        let mut data10 = vec![5; 80];
        let mut lamports10 = 42;
        let pid = WORMHOLE_TOKEN_BRIDGE_PROGRAM_ID;
        let sysvar_id = sysvar::id();
        let accts = TransactionAccountKeys {
            core_bridge_config: core_bridge_config(),
            core_message_account: core_message_account(pid, 69),
            emitter: emitter(pid),
            core_emitter_sequence: core_emitter_sequence(emitter(pid)),
            payer: payer(),
            core_fee_collector: core_fee_collector(),
            clock: sysvar::clock::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
            core_bridge_program: WORMHOLE_PROGRAM_ID,
        };
        let core_bridge_config = AccountInfo::new(
            &accts.core_bridge_config,
            false,
            false,
            &mut lamports,
            &mut data,
            &WORMHOLE_PROGRAM_ID,
            false,
            0,
        );
        let core_message_account = AccountInfo::new(
            &accts.core_message_account,
            false,
            false,
            &mut lamports2,
            &mut data2,
            &key,
            false,
            0,
        );
        let emitter = AccountInfo::new(
            &accts.emitter,
            false,
            false,
            &mut lamports3,
            &mut data3,
            &pid,
            false,
            0,
        );
        let core_emitter_sequence = AccountInfo::new(
            &accts.core_emitter_sequence,
            false,
            false,
            &mut lamports4,
            &mut data4,
            &WORMHOLE_PROGRAM_ID,
            false,
            0,
        );
        let payer = AccountInfo::new(
            &accts.payer,
            false,
            false,
            &mut lamports5,
            &mut data5,
            &key,
            false,
            0,
        );
        let core_fee_collector = AccountInfo::new(
            &accts.core_fee_collector,
            false,
            false,
            &mut lamports6,
            &mut data6,
            &WORMHOLE_PROGRAM_ID,
            false,
            0,
        );
        let clock = AccountInfo::new(
            &accts.clock,
            false,
            false,
            &mut lamports7,
            &mut data7,
            &sysvar_id,
            false,
            0,
        );
        let system_program = AccountInfo::new(
            &accts.system_program,
            false,
            false,
            &mut lamports8,
            &mut data8,
            &key,
            false,
            0,
        );
        let rent = AccountInfo::new(
            &accts.rent,
            false,
            false,
            &mut lamports9,
            &mut data9,
            &sysvar_id,
            false,
            0,
        );
        let core_bridge_program = AccountInfo::new(
            &WORMHOLE_PROGRAM_ID,
            false,
            false,
            &mut lamports10,
            &mut data10,
            &WORMHOLE_PROGRAM_ID,
            false,
            0,
        );

        let account_infos_vec = vec![
            core_bridge_config.clone(),
            core_message_account.clone(),
            emitter.clone(),
            core_emitter_sequence.clone(),
            payer.clone(),
            core_fee_collector.clone(),
            clock.clone(),
            system_program.clone(),
            rent.clone(),
            core_bridge_program.clone(),
        ];

        let accounts: Accounts<'_> = Accounts::from(&account_infos_vec[..]);

        assert_eq!(*accounts.core_bridge_config.key, accts.core_bridge_config);
        assert_eq!(
            *accounts.core_message_account.key,
            accts.core_message_account
        );
        assert_eq!(*accounts.emitter.key, accts.emitter);
        assert_eq!(
            *accounts.core_emitter_sequence.key,
            accts.core_emitter_sequence
        );
        assert_eq!(*accounts.payer.key, accts.payer);
        assert_eq!(*accounts.core_fee_collector.key, accts.core_fee_collector);
        assert_eq!(*accounts.clock.key, accts.clock);
        assert_eq!(*accounts.system_program.key, accts.system_program);
        assert_eq!(*accounts.rent.key, accts.rent);
        assert_eq!(*accounts.core_bridge_program.key, accts.core_bridge_program);

        for (a1, a2) in accounts.to_vec().iter().zip(account_infos_vec.iter()) {
            assert_eq!(a1.key, a2.key);
        }
        assert!(accounts.validate(
            accts.emitter,
            accts.core_message_account,
            accts.core_emitter_sequence,
            pid,
        ));
        assert!(!accounts.validate(
            accts.emitter,
            accts.core_message_account,
            accts.core_emitter_sequence,
            Pubkey::new_unique(),
        ));
        let fee_collector_ix = accounts.fee_collector_ix();
        assert_eq!(
            fee_collector_ix,
            Instruction::new_with_bincode(
                system_program::id(),
                &SystemInstruction::Transfer { lamports: 100 },
                vec![
                    AccountMeta::new(*accounts.payer.key, true),
                    AccountMeta::new(*accounts.core_fee_collector.key, false)
                ]
            )
        );
        let post_msg_ix =
            accounts.post_message_ix(69, b"Hello World".to_vec(), Finality::Finalized);
        assert_eq!(
            post_msg_ix,
            Instruction {
                program_id: WORMHOLE_PROGRAM_ID,
                accounts: accts.to_account_metas(),
                data: wormhole_anchor_sdk::wormhole::Instruction::PostMessage {
                    batch_id: 69,
                    payload: b"Hello World".to_vec(),
                    finality: Finality::Finalized
                }
                .try_to_vec()
                .unwrap()
            }
        )
    }
}
