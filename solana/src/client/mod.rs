//! offchain rpc client library

/// helpers for working with the solana secp256k1 program
pub mod secp256k1_helpers;

/// creates the transaction bundle needed to verify a signed VAA
pub mod vaa_verification_bundle;
