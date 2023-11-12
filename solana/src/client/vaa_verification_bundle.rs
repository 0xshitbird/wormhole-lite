use crate::instructions::verify_signature::{
    create_verify_signature_ix, VerifySignaturesData, MAX_LEN_GUARDIAN_KEYS,
};
use anyhow::Context;
use borsh::BorshDeserialize;
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::transaction::Transaction;
use wormhole_core_bridge_solana::state::GuardianSet;
use wormhole_explorer_client::{self, endpoints::vaa::ExplorerVaa};

use crate::client::secp256k1_helpers::{make_secp256k1_instruction_data, SecpSignature};

/// numbers of signatures permitted in a single batch of verification
pub const BATCH_SIZE: usize = 7;

/// contains the start, and end indices of the the signed vaa guardian_set
/// that are to be used in a verify_signature instruction
pub struct SignatureBatchParameters {
    pub start: usize,
    pub end: usize,
}

/// Contains all the needed instructions to verify a VAA on-chain
/// before it can be consumed. This must be done in two transactiosn
/// which must be executed based on the order of the fields tx<N>/
///
/// The transactions must be signed by the public key that was specified
/// as the fee payer before they can be broadcast
#[derive(Clone, Default)]
pub struct VaaSignatureVerificationBundle {
    pub txs: Vec<Transaction>,
}

/// parses a wormhole VAA into the instructions needed to verify it on chain
/// before it can be posted for consumption
pub async fn create_vaa_verification_instructions(
    // the account which will be paying transaction fees
    payer: Pubkey,
    // the account which will store signature verification data onchain
    wormhole_signature_account: Pubkey,
    rpc: &solana_client::nonblocking::rpc_client::RpcClient,
    explorer_vaa: &ExplorerVaa,
) -> anyhow::Result<VaaSignatureVerificationBundle> {
    let deser_vaa = explorer_vaa.deser_vaa()?;
    let signature_length = deser_vaa.header.signatures.len();
    let verification_hash = deser_vaa.body.digest();
    let (guardian_set_key, _) =
        crate::utils::derivations::derive_guardian_set(deser_vaa.header.guardian_set_index);
    let mut guardian_set = load_guardian_set_account(guardian_set_key, rpc).await?;

    let batches = get_batches(deser_vaa.header.signatures.len());

    let mut tx_bundle = VaaSignatureVerificationBundle::new(batches);

    for i in 0..batches {
        let batch_params = SignatureBatchParameters::new(i, signature_length);
        // used to indicate which guardians of the wormhole network's list of all guardians
        // that were involved in signing the vaa
        let mut signature_status: [i8; MAX_LEN_GUARDIAN_KEYS] = [-1_i8; MAX_LEN_GUARDIAN_KEYS];
        // holds each individual guardian's signature of the vaa
        let mut signatures = Vec::with_capacity(BATCH_SIZE);
        // public keys of guardians
        let mut guardian_keys = Vec::with_capacity(BATCH_SIZE);
        // contains signature information in the format needed by the secp256k1 program
        let mut secp_signatures = Vec::with_capacity(BATCH_SIZE);
        for j in 0..(batch_params.end - batch_params.start) {
            let guardian_signature = &deser_vaa.header.signatures[j + batch_params.start];
            // set the sig verification status based on the index of the guardian
            // in the actual gaurdian_set account, where this is used by the
            // wormhole program verify_signatures function
            signature_status[guardian_signature.guardian_set_index as usize] = j as i8;
            // this sets the signature of the guardian based on the order in which they
            // signed the vaa, this is used for the secp256k1 program instruction
            signatures.push(guardian_signature.signature);
            // guardian set keys are stored as a vector and don't need to be used after this, so we can avoid the clone
            let guardian_key = std::mem::take(
                &mut guardian_set.keys[guardian_signature.guardian_set_index as usize],
            );
            guardian_keys.push(guardian_key);
            secp_signatures.push(SecpSignature {
                signature: guardian_signature.raw_sig(),
                recovery_id: guardian_signature.recovery_id(),
                eth_address: guardian_key,
                message: verification_hash.0,
            })
        }
        // we will always be executing this in instruction index 0 due to requirements of wormhole's verify_signature instruction
        let secp_instruction_data = make_secp256k1_instruction_data(&secp_signatures, 0)?;
        let secp256k1_ix = Instruction::new_with_bytes(
            solana_sdk::secp256k1_program::ID,
            &secp_instruction_data,
            vec![],
        );
        let verify_sig_ix = create_verify_signature_ix(
            payer,
            deser_vaa.header.guardian_set_index,
            wormhole_signature_account,
            VerifySignaturesData {
                signers: signature_status,
            },
        )
        .with_context(|| "failed to create verify_signature instruction")?;
        let tx = Transaction::new_with_payer(&[secp256k1_ix, verify_sig_ix], Some(&payer));
        tx_bundle.txs.push(tx);
    }

    Ok(tx_bundle)
}

/// loads the guardian set account which contains the actual public keys
/// of the guardians that were used to verify sign the VAA
pub async fn load_guardian_set_account(
    key: Pubkey,
    rpc: &solana_client::nonblocking::rpc_client::RpcClient,
) -> anyhow::Result<GuardianSet> {
    let account_data = rpc
        .get_account_data(&key)
        .await
        .with_context(|| "failed to get account data")?;
    GuardianSet::try_from_slice(&account_data[..]).with_context(|| "failed to parse account data")
}

/// returns the number of batched secp256k1 ix + verify_signature ix that must be
/// sent before a VAA can be posted
pub fn get_batches(signature_length: usize) -> usize {
    (signature_length as f64 / BATCH_SIZE as f64).ceil() as usize
}

impl SignatureBatchParameters {
    pub fn new(loop_iteration: usize, signature_length: usize) -> Self {
        Self {
            start: loop_iteration * BATCH_SIZE,
            end: usize::min(signature_length, (loop_iteration + 1) * BATCH_SIZE),
        }
    }
}

impl VaaSignatureVerificationBundle {
    pub fn new(batch_size: usize) -> Self {
        Self {
            txs: Vec::with_capacity(batch_size),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_get_batches() {
        let num_batches = get_batches(13);
        assert_eq!(num_batches, 2);
    }
    #[tokio::test]
    async fn test_load_guardian_set_account() {
        let rpc = solana_client::nonblocking::rpc_client::RpcClient::new("https://quiet-cool-waterfall.solana-mainnet.quiknode.pro/3b7848ce3a28b7de7fe04739500e9d50b906cae4/".to_string());
        let (guardian_key, _) = crate::utils::derivations::derive_guardian_set(3);
        let guardian_set = load_guardian_set_account(guardian_key, &rpc).await.unwrap();
        println!("{:#?}", guardian_set);
    }
}
