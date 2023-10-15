use solana_program::pubkey::Pubkey;

/// state objects for solana programs
pub mod state;

/// utilities for working with wormhole on solana
pub mod utils;

/// instructions for invoking the wormhole bridge program through cpi
pub mod instructions;

/// structured payloads for handling arbitrary messages
pub mod message_payload;

/// id of the core wormhole program
pub const WORMHOLE_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("worm2ZoG2kUd4vFXhvjh93UUH596ayRfgQ2MgjNMTth");
/// id of the token bridge core wrapper
pub const WORMHOLE_TOKEN_BRIDGE_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("wormDTUJ6AWPNvk59vGQbDvGJmqbDTdgWgAqcLBCgUb");
/// id of the nft bridge core wrapepr
pub const WORMHOLE_NFT_BRIDGE_PROGRAM_ID: Pubkey =
    solana_program::pubkey!("WnFt12ZrnzZrFZkt2xsNsaNWoQribnuQ5B5FrDbwDhD");
