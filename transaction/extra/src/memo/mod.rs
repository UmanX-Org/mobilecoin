// Copyright (c) 2018-2023 The MobileCoin Foundation

//! Defines an object for each known high-level memo type,
//! and an enum to allow matching recovered memos to one of these types.
//!
//! The framework for memo types was proposed for standardization
//! in mobilecoinfoundation/mcips/pull/3.
//!
//! Several memo types from that proposal and subsequent proposals
//! are implemented in modules beneath this module, and then included in the
//! MemoType enum defined in this module.
//!
//! The intended use is like:
//! - Call `TxOut::decrypt_memo`, obtaining `MemoPayload`
//! - Call `MemoType::try_from`, obtaining the enum `MemoType`
//! - Match on the enum, which tells you what memo type this is, then you can
//!   read that data and validate it. See individual memo types for their
//!   semantics.
//!
//! To add a new memo type, you can add it to this crate in a new module,
//! make it implement `RegisteredMemoType`, and add it to the `impl_memo_enum`
//! macro call below.
//!
//! You can also make your own custom version of `MemoType` using different
//! structs, in your own crate, if you prefer. The `impl_memo_enum` macro is
//! exported, and will work as long as your memo types all implement
//! RegisteredMemoType, and all have different MEMO_TYPE_BYTES.
//!
//! If you want to put new memo types into transactions, you will need to
//! implement a new `MemoBuilder`. See the `memo_builder` module for examples.
//! Or, if you don't want to use the `TransactionBuilder`, you can call
//! `TxOut::new_with_memo` directly.
//!
//! The following memo types are natively supported by this module:
//! | Memo type bytes | Name                                              |
//! | -----------     | -----------                                       |
//! | 0x0000          | Unused                                            |
//! | 0x0001          | Burn Redemption Memo                              |
//! | 0x0002          | Gift Code Sender Memo                             |
//! | 0x0003          | Defragmentation Memo                              |
//! | 0x0100          | Authenticated Sender Memo                         |
//! | 0x0101          | Authenticated Sender With Payment Request Id Memo |
//! | 0x0102          | Authenticated Sender With Payment Intent Id Memo  |
//! | 0x0200          | Destination Memo                                  |
//! | 0x0201          | Gift Code Funding Memo                            |
//! | 0x0202          | Gift Code Cancellation Memo                       |
//! | 0x0203          | Destination With Payment Request Id Memo          |
//! | 0x0204          | Destination With Payment Intent Id Memo           |

pub use self::{
    authenticated_common::compute_authenticated_sender_memo,
    authenticated_sender::AuthenticatedSenderMemo,
    authenticated_sender_with_payment_intent_id::AuthenticatedSenderWithPaymentIntentIdMemo,
    authenticated_sender_with_payment_request_id::AuthenticatedSenderWithPaymentRequestIdMemo,
    burn_redemption::BurnRedemptionMemo,
    credential::SenderMemoCredential,
    defragmentation::{DefragmentationMemo, DefragmentationMemoError},
    destination::{compute_destination_memo, DestinationMemo, DestinationMemoError},
    destination_with_payment_intent_id::DestinationWithPaymentIntentIdMemo,
    destination_with_payment_request_id::DestinationWithPaymentRequestIdMemo,
    gift_code_cancellation::GiftCodeCancellationMemo,
    gift_code_funding::GiftCodeFundingMemo,
    gift_code_sender::GiftCodeSenderMemo,
    unused::UnusedMemo,
};

mod authenticated_common;
mod authenticated_sender;
mod authenticated_sender_with_payment_intent_id;
mod authenticated_sender_with_payment_request_id;
mod burn_redemption;
mod credential;
mod defragmentation;
mod destination;
mod destination_with_payment_intent_id;
mod destination_with_payment_request_id;
mod gift_code_cancellation;
mod gift_code_funding;
mod gift_code_sender;
mod macros;
mod unused;

use crate::impl_memo_enum;
use core::fmt::Debug;
use displaydoc::Display;

/// A trait that all registered memo types should implement.
/// This creates a single source of truth for the memo type bytes.
pub trait RegisteredMemoType:
    Sized + Clone + Debug + Into<[u8; 64]> + for<'a> From<&'a [u8; 64]>
{
    /// The type bytes assigned to this memo type.
    /// These are typically found in the MCIP that specifies this memo type.
    ///
    /// The first byte is conceptually a "type category"
    /// The second byte is a type within the category
    const MEMO_TYPE_BYTES: [u8; 2];
}

/// An error that can occur when trying to interpret a raw MemoPayload as
/// a MemoType
#[derive(Clone, Display, Debug)]
pub enum MemoDecodingError {
    /// Unknown memo type: type bytes were {0:02X?}
    UnknownMemoType([u8; 2]),
}

impl_memo_enum! { MemoType,
    AuthenticatedSender(AuthenticatedSenderMemo), //[0x01, 0x00]
    AuthenticatedSenderWithPaymentRequestId(AuthenticatedSenderWithPaymentRequestIdMemo), //[0x01, 0x01]
    AuthenticatedSenderWithPaymentIntentId(AuthenticatedSenderWithPaymentIntentIdMemo), //[0x01, 0x02]
    BurnRedemption(BurnRedemptionMemo), //[0x00, 0x01]
    Defragmentation(DefragmentationMemo), //[0x00, 0x03]
    Destination(DestinationMemo), //[0x02, 0x00]
    DestinationWithPaymentRequestId(DestinationWithPaymentRequestIdMemo), //[0x02, 0x03]
    DestinationWithPaymentIntentId(DestinationWithPaymentIntentIdMemo), //[0x02, 0x04]
    GiftCodeCancellation(GiftCodeCancellationMemo), //[0x02, 0x02]
    GiftCodeFunding(GiftCodeFundingMemo), //[0x02, 0x01]
    GiftCodeSender(GiftCodeSenderMemo), //[0x00, 0x02]
    Unused(UnusedMemo), //[0x00, 0x00]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_account_keys::{AccountKey, ShortAddressHash};
    use mc_crypto_keys::{CompressedRistrettoPublic, RistrettoPrivate};
    use mc_transaction_core::MemoPayload;
    use mc_util_from_random::FromRandom;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn test_memo_type_round_trips() {
        let mut rng: StdRng = SeedableRng::from_seed([1u8; 32]);

        let alice = AccountKey::new(
            &RistrettoPrivate::from_random(&mut rng),
            &RistrettoPrivate::from_random(&mut rng),
        );
        let alice_cred = SenderMemoCredential::from(&alice);

        let bob = AccountKey::new(
            &RistrettoPrivate::from_random(&mut rng),
            &RistrettoPrivate::from_random(&mut rng),
        );
        let bob_addr = bob.default_subaddress();

        let tx_public_key = CompressedRistrettoPublic::from_random(&mut rng);

        let memo1 = UnusedMemo {};
        match MemoType::try_from(&MemoPayload::from(memo1)).unwrap() {
            MemoType::Unused(_) => {}
            _ => {
                panic!("unexpected deserialization");
            }
        }

        let memo2 =
            AuthenticatedSenderMemo::new(&alice_cred, bob_addr.view_public_key(), &tx_public_key);
        match MemoType::try_from(&MemoPayload::from(memo2.clone())).unwrap() {
            MemoType::AuthenticatedSender(memo) => {
                assert_eq!(memo2, memo, "memo did not round trip");
            }
            _ => {
                panic!("unexpected deserialization");
            }
        }

        let memo3 = AuthenticatedSenderWithPaymentRequestIdMemo::new(
            &alice_cred,
            bob_addr.view_public_key(),
            &tx_public_key,
            7u64,
        );
        match MemoType::try_from(&MemoPayload::from(memo3.clone())).unwrap() {
            MemoType::AuthenticatedSenderWithPaymentRequestId(memo) => {
                assert_eq!(memo3, memo);
            }
            _ => {
                panic!("unexpected deserialization");
            }
        }

        let memo4 = DestinationMemo::new(ShortAddressHash::from(&bob_addr), 17, 18).unwrap();
        match MemoType::try_from(&MemoPayload::from(memo4.clone())).unwrap() {
            MemoType::Destination(memo) => {
                assert_eq!(memo4, memo);
            }
            _ => {
                panic!("unexpected deserialization");
            }
        }

        let memo5 = MemoPayload::new([7u8, 8u8], [0u8; 64]);
        match MemoType::try_from(&memo5) {
            Ok(_) => {
                panic!("failure was expected");
            }
            Err(err) => match err {
                MemoDecodingError::UnknownMemoType(code) => {
                    assert_eq!(code, [7u8, 8u8], "unexpected memo type bytes");
                }
            },
        }

        let memo6 = BurnRedemptionMemo::new([2; 64]);
        match MemoType::try_from(&MemoPayload::from(memo6.clone())).unwrap() {
            MemoType::BurnRedemption(memo) => {
                assert_eq!(memo6, memo);
            }
            _ => {
                panic!("unexpected deserialization");
            }
        }
    }

    #[test]
    fn test_memo_authentication() {
        let mut rng: StdRng = SeedableRng::from_seed([2u8; 32]);

        let alice = AccountKey::new(
            &RistrettoPrivate::from_random(&mut rng),
            &RistrettoPrivate::from_random(&mut rng),
        );
        let alice_cred = SenderMemoCredential::from(&alice);
        let alice_addr = alice.default_subaddress();

        let bob = AccountKey::new(
            &RistrettoPrivate::from_random(&mut rng),
            &RistrettoPrivate::from_random(&mut rng),
        );
        let bob_addr = bob.default_subaddress();

        let tx_public_key = CompressedRistrettoPublic::from_random(&mut rng);
        let tx_public_key2 = CompressedRistrettoPublic::from_random(&mut rng);

        let memo1 =
            AuthenticatedSenderMemo::new(&alice_cred, bob_addr.view_public_key(), &tx_public_key);
        assert_eq!(
            memo1.sender_address_hash(),
            ShortAddressHash::from(&alice_addr)
        );
        assert!(
            bool::from(memo1.validate(
                &alice_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have passed"
        );
        assert!(
            !bool::from(memo1.validate(
                &bob_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo1.validate(
                &alice_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo1.validate(
                &bob_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo1.validate(
                &alice_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo1.validate(
                &bob_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo1.validate(
                &alice_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo1.validate(
                &bob_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );

        let memo2 = AuthenticatedSenderWithPaymentRequestIdMemo::new(
            &alice_cred,
            bob_addr.view_public_key(),
            &tx_public_key,
            7u64,
        );
        assert_eq!(
            memo2.sender_address_hash(),
            ShortAddressHash::from(&alice_addr)
        );
        assert_eq!(memo2.payment_request_id(), 7u64);
        assert!(
            bool::from(memo2.validate(
                &alice_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have passed"
        );
        assert!(
            !bool::from(memo2.validate(
                &bob_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo2.validate(
                &alice_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo2.validate(
                &bob_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo2.validate(
                &alice_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo2.validate(
                &bob_addr,
                &bob.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo2.validate(
                &alice_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
        assert!(
            !bool::from(memo2.validate(
                &bob_addr,
                &alice.default_subaddress_view_private(),
                &tx_public_key2
            )),
            "validation should have failed"
        );
    }

    #[test]
    fn test_destination_memo() {
        let mut rng: StdRng = SeedableRng::from_seed([2u8; 32]);

        let alice = AccountKey::new(
            &RistrettoPrivate::from_random(&mut rng),
            &RistrettoPrivate::from_random(&mut rng),
        );
        let alice_addr = alice.default_subaddress();

        let bob = AccountKey::new(
            &RistrettoPrivate::from_random(&mut rng),
            &RistrettoPrivate::from_random(&mut rng),
        );
        let bob_addr = bob.default_subaddress();

        let mut memo =
            DestinationMemo::new(ShortAddressHash::from(&alice_addr), 12u64, 13u64).unwrap();

        assert_eq!(
            memo.get_address_hash(),
            &ShortAddressHash::from(&alice_addr)
        );
        assert_eq!(memo.get_total_outlay(), 12u64);
        assert_eq!(memo.get_fee(), 13u64);
        assert_eq!(memo.get_num_recipients(), 1);

        memo.set_address_hash(ShortAddressHash::from(&bob_addr));
        memo.set_total_outlay(19);
        memo.set_fee(17).unwrap();
        memo.set_num_recipients(4);

        assert_eq!(memo.get_address_hash(), &ShortAddressHash::from(&bob_addr));
        assert_eq!(memo.get_total_outlay(), 19u64);
        assert_eq!(memo.get_fee(), 17u64);
        assert_eq!(memo.get_num_recipients(), 4);
    }
}
