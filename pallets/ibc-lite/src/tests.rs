//! IBC-lite pallet unit tests.

#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok};

// =========================================================
// Helper Functions
// =========================================================

fn open_channel_helper(channel_num: u64) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let counterparty_chain = format!("chain-{}", channel_num).into_bytes();
    let counterparty_channel = format!("remote-channel-{}", channel_num).into_bytes();
    let expected_channel_id = format!("channel-{}", channel_num).into_bytes();

    assert_ok!(IbcLite::open_channel(
        frame_system::RawOrigin::Root.into(),
        counterparty_chain.clone(),
        counterparty_channel.clone(),
    ));

    // Manually transition to Open for testing
    let channel_id: ChannelId<Runtime> = expected_channel_id.clone().try_into().unwrap();
    Channels::<Runtime>::mutate(&channel_id, |maybe_channel| {
        if let Some(ch) = maybe_channel {
            ch.state = ChannelState::Open;
        }
    });

    (
        expected_channel_id,
        counterparty_chain,
        counterparty_channel,
    )
}

// =========================================================
// Channel Tests
// =========================================================

#[test]
fn open_channel_works() {
    new_test_ext().execute_with(|| {
        let (channel_id, chain, remote) = open_channel_helper(0);

        // Verify channel was created
        let bounded_id: ChannelId<Runtime> = channel_id.clone().try_into().unwrap();
        let channel = Channels::<Runtime>::get(&bounded_id).unwrap();
        assert_eq!(channel.state, ChannelState::Open);
        assert_eq!(channel.channel_id.to_vec(), channel_id);

        // Verify it's in the chain index
        let chain_id: ChainId<Runtime> = chain.try_into().unwrap();
        let channels = ChannelsByChain::<Runtime>::get(&chain_id);
        assert!(channels.contains(&bounded_id));
    });
}

#[test]
fn open_channel_requires_authorized_origin() {
    new_test_ext().execute_with(|| {
        assert_err!(
            IbcLite::open_channel(
                frame_system::RawOrigin::Signed(1).into(),
                b"chain-0".to_vec(),
                b"remote-channel-0".to_vec(),
            ),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn close_channel_init_works() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        assert_ok!(IbcLite::close_channel_init(
            frame_system::RawOrigin::Root.into(),
            channel_id.clone(),
        ));

        let bounded_id: ChannelId<Runtime> = channel_id.try_into().unwrap();
        let channel = Channels::<Runtime>::get(&bounded_id).unwrap();
        assert_eq!(channel.state, ChannelState::CloseInit);
    });
}

#[test]
fn close_channel_confirm_requires_trusted_relayer() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        // Add a relayer first
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert_err!(
            IbcLite::close_channel_confirm(frame_system::RawOrigin::Signed(1).into(), channel_id,),
            Error::<Runtime>::NotTrustedRelayer
        );
    });
}

#[test]
fn close_channel_confirm_completes_close() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        // Add relayer and initiate close
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));
        assert_ok!(IbcLite::close_channel_init(
            frame_system::RawOrigin::Root.into(),
            channel_id.clone(),
        ));

        // Confirm close
        assert_ok!(IbcLite::close_channel_confirm(
            frame_system::RawOrigin::Signed(10).into(),
            channel_id.clone(),
        ));

        let bounded_id: ChannelId<Runtime> = channel_id.try_into().unwrap();
        let channel = Channels::<Runtime>::get(&bounded_id).unwrap();
        assert_eq!(channel.state, ChannelState::Closed);
    });
}

#[test]
fn cannot_send_on_closed_channel() {
    new_test_ext().execute_with(|| {
        let (channel_id, chain, _) = open_channel_helper(0);

        // Close the channel
        let bounded_id: ChannelId<Runtime> = channel_id.clone().try_into().unwrap();
        Channels::<Runtime>::mutate(&bounded_id, |maybe_ch| {
            if let Some(ch) = maybe_ch {
                ch.state = ChannelState::Closed;
            }
        });

        // Try to send
        assert_err!(
            IbcLite::send_packet(
                frame_system::RawOrigin::Signed(1).into(),
                channel_id.clone(),
                chain,
                b"remote-channel-0".to_vec(),
                None,
                PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
            ),
            Error::<Runtime>::ChannelNotOpen
        );
    });
}

// =========================================================
// Packet Tests
// =========================================================

#[test]
fn send_packet_works() {
    new_test_ext().execute_with(|| {
        let (channel_id, chain, remote) = open_channel_helper(0);

        assert_ok!(IbcLite::send_packet(
            frame_system::RawOrigin::Signed(1).into(),
            channel_id.clone(),
            chain,
            remote,
            None,
            PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
        ));

        // Check commitment was stored
        let bounded_id: ChannelId<Runtime> = channel_id.try_into().unwrap();
        assert!(PacketCommitments::<Runtime>::contains_key(&bounded_id, 1));

        // Check sequence incremented
        assert_eq!(SendSequences::<Runtime>::get(&bounded_id), 2);
    });
}

#[test]
fn send_packet_fails_if_channel_not_open() {
    new_test_ext().execute_with(|| {
        let (channel_id, chain, remote) = open_channel_helper(0);

        // Close the channel
        let bounded_id: ChannelId<Runtime> = channel_id.clone().try_into().unwrap();
        Channels::<Runtime>::mutate(&bounded_id, |maybe_ch| {
            if let Some(ch) = maybe_ch {
                ch.state = ChannelState::Closed;
            }
        });

        assert_err!(
            IbcLite::send_packet(
                frame_system::RawOrigin::Signed(1).into(),
                channel_id,
                chain,
                remote,
                None,
                PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
            ),
            Error::<Runtime>::ChannelNotOpen
        );
    });
}

#[test]
fn receive_packet_works() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, remote) = open_channel_helper(0);

        // Add relayer
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        let bounded_id: ChannelId<Runtime> = channel_id.clone().try_into().unwrap();
        let remote_bounded: ChannelId<Runtime> = remote.try_into().unwrap();

        let packet = Packet::<Runtime> {
            sequence: 1,
            src_channel_id: remote_bounded,
            dst_channel_id: bounded_id.clone(),
            dst_chain_id: b"clawchain".to_vec().try_into().unwrap(),
            src_agent_id: None,
            dst_agent_id: None,
            payload: PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
            timeout_height: 1000u32.into(),
            created_at: 100u32.into(),
        };

        assert_ok!(IbcLite::receive_packet(
            frame_system::RawOrigin::Signed(10).into(),
            packet,
        ));

        // Check receipt was stored
        assert!(PacketReceipts::<Runtime>::contains_key(&bounded_id, 1));

        // Check sequence incremented
        assert_eq!(RecvSequences::<Runtime>::get(&bounded_id), 2);
    });
}

#[test]
fn receive_packet_rejects_replay() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, remote) = open_channel_helper(0);

        // Add relayer and send a packet
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        let bounded_id: ChannelId<Runtime> = channel_id.clone().try_into().unwrap();
        let remote_bounded: ChannelId<Runtime> = remote.try_into().unwrap();

        let packet = Packet::<Runtime> {
            sequence: 1,
            src_channel_id: remote_bounded,
            dst_channel_id: bounded_id.clone(),
            dst_chain_id: b"clawchain".to_vec().try_into().unwrap(),
            src_agent_id: None,
            dst_agent_id: None,
            payload: PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
            timeout_height: 1000u32.into(),
            created_at: 100u32.into(),
        };

        // First receive
        assert_ok!(IbcLite::receive_packet(
            frame_system::RawOrigin::Signed(10).into(),
            packet.clone(),
        ));

        // Second receive should fail
        assert_err!(
            IbcLite::receive_packet(frame_system::RawOrigin::Signed(10).into(), packet,),
            Error::<Runtime>::PacketAlreadyReceived
        );
    });
}

#[test]
fn receive_packet_rejects_non_relayer() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, remote) = open_channel_helper(0);

        let bounded_id: ChannelId<Runtime> = channel_id.try_into().unwrap();
        let remote_bounded: ChannelId<Runtime> = remote.try_into().unwrap();

        let packet = Packet::<Runtime> {
            sequence: 1,
            src_channel_id: remote_bounded,
            dst_channel_id: bounded_id,
            dst_chain_id: b"clawchain".to_vec().try_into().unwrap(),
            src_agent_id: None,
            dst_agent_id: None,
            payload: PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
            timeout_height: 1000u32.into(),
            created_at: 100u32.into(),
        };

        assert_err!(
            IbcLite::receive_packet(frame_system::RawOrigin::Signed(1).into(), packet,),
            Error::<Runtime>::NotTrustedRelayer
        );
    });
}

#[test]
fn acknowledge_packet_works() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        // Add relayer
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        // Send a packet to create commitment
        assert_ok!(IbcLite::send_packet(
            frame_system::RawOrigin::Signed(1).into(),
            channel_id.clone(),
            b"chain-0".to_vec(),
            b"remote-channel-0".to_vec(),
            None,
            PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
        ));

        // Acknowledge
        assert_ok!(IbcLite::acknowledge_packet(
            frame_system::RawOrigin::Signed(10).into(),
            channel_id.clone(),
            1,
            PacketPayload::Ack {
                success: true,
                error_code: None,
                data: vec![].try_into().unwrap(),
            },
        ));

        let bounded_id: ChannelId<Runtime> = channel_id.try_into().unwrap();

        // Commitment should be deleted
        assert!(!PacketCommitments::<Runtime>::contains_key(&bounded_id, 1));

        // Ack should be stored
        assert!(PacketAcknowledgements::<Runtime>::contains_key(
            &bounded_id,
            1
        ));
    });
}

#[test]
fn acknowledge_packet_fails_if_no_commitment() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        // Add relayer
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert_err!(
            IbcLite::acknowledge_packet(
                frame_system::RawOrigin::Signed(10).into(),
                channel_id,
                1,
                PacketPayload::Ack {
                    success: true,
                    error_code: None,
                    data: vec![].try_into().unwrap(),
                },
            ),
            Error::<Runtime>::PacketNotFound
        );
    });
}

#[test]
fn timeout_packet_works() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        // Send a packet
        assert_ok!(IbcLite::send_packet(
            frame_system::RawOrigin::Signed(1).into(),
            channel_id.clone(),
            b"chain-0".to_vec(),
            b"remote-channel-0".to_vec(),
            None,
            PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
        ));

        // Advance to block >= PacketTimeoutBlocks (100) so the timeout has elapsed.
        // This is required after the C1 security fix: timeout_packet now verifies
        // the timeout height before allowing cancellation.
        frame_system::Pallet::<Runtime>::set_block_number(101);

        // Timeout
        assert_ok!(IbcLite::timeout_packet(
            frame_system::RawOrigin::Signed(1).into(),
            channel_id.clone(),
            1,
        ));

        let bounded_id: ChannelId<Runtime> = channel_id.try_into().unwrap();

        // Commitment should be deleted
        assert!(!PacketCommitments::<Runtime>::contains_key(&bounded_id, 1));
    });
}

/// Security regression: C1 fix — premature timeout must be rejected.
/// Before the fix, `timeout_packet` had no timeout height check and could be
/// called on any pending packet immediately after send.
#[test]
fn timeout_packet_rejected_before_timeout() {
    new_test_ext().execute_with(|| {
        let (channel_id, _, _) = open_channel_helper(0);

        // Send a packet (timeout_height = block 1 + 100 = 101)
        assert_ok!(IbcLite::send_packet(
            frame_system::RawOrigin::Signed(1).into(),
            channel_id.clone(),
            b"chain-0".to_vec(),
            b"remote-channel-0".to_vec(),
            None,
            PacketPayload::Raw(vec![1, 2, 3].try_into().unwrap()),
        ));

        // Attempting timeout at block 1 (before block 101) must fail
        assert_noop!(
            IbcLite::timeout_packet(
                frame_system::RawOrigin::Signed(1).into(),
                channel_id.clone(),
                1,
            ),
            Error::<Runtime>::PacketTimedOut,
        );
    });
}

// =========================================================
// Relayer Management Tests
// =========================================================

#[test]
fn add_relayer_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert!(TrustedRelayers::<Runtime>::get().contains(&10));
    });
}

#[test]
fn add_relayer_requires_manager_origin() {
    new_test_ext().execute_with(|| {
        assert_err!(
            IbcLite::add_relayer(frame_system::RawOrigin::Signed(1).into(), 10,),
            sp_runtime::DispatchError::BadOrigin
        );
    });
}

#[test]
fn add_relayer_rejects_duplicate() {
    new_test_ext().execute_with(|| {
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert_err!(
            IbcLite::add_relayer(frame_system::RawOrigin::Root.into(), 10,),
            Error::<Runtime>::RelayerAlreadyRegistered
        );
    });
}

#[test]
fn remove_relayer_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert_ok!(IbcLite::remove_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert!(!TrustedRelayers::<Runtime>::get().contains(&10));
    });
}

#[test]
fn remove_relayer_fails_if_not_registered() {
    new_test_ext().execute_with(|| {
        assert_err!(
            IbcLite::remove_relayer(frame_system::RawOrigin::Root.into(), 10,),
            Error::<Runtime>::RelayerNotFound
        );
    });
}

// =========================================================
// Cross-Chain Agent Tests
// =========================================================

#[test]
fn register_cross_chain_agent_works() {
    new_test_ext().execute_with(|| {
        // Add relayer
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert_ok!(IbcLite::register_cross_chain_agent(
            frame_system::RawOrigin::Signed(10).into(),
            b"remote-chain".to_vec(),
            b"remote-agent-1".to_vec(),
            5, // Valid agent ID (1-100 in mock)
        ));

        let chain_id: ChainId<Runtime> = b"remote-chain".to_vec().try_into().unwrap();
        let agent_id: RemoteAgentId<Runtime> = b"remote-agent-1".to_vec().try_into().unwrap();

        assert_eq!(
            CrossChainAgentMap::<Runtime>::get(&chain_id, &agent_id),
            Some(5)
        );
    });
}

#[test]
fn register_cross_chain_agent_rejects_duplicate() {
    new_test_ext().execute_with(|| {
        // Add relayer
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        assert_ok!(IbcLite::register_cross_chain_agent(
            frame_system::RawOrigin::Signed(10).into(),
            b"remote-chain".to_vec(),
            b"remote-agent-1".to_vec(),
            5,
        ));

        assert_err!(
            IbcLite::register_cross_chain_agent(
                frame_system::RawOrigin::Signed(10).into(),
                b"remote-chain".to_vec(),
                b"remote-agent-1".to_vec(),
                6,
            ),
            Error::<Runtime>::CrossChainAgentAlreadyMapped
        );
    });
}

#[test]
fn register_cross_chain_agent_validates_agent_exists() {
    new_test_ext().execute_with(|| {
        // Add relayer
        assert_ok!(IbcLite::add_relayer(
            frame_system::RawOrigin::Root.into(),
            10,
        ));

        // Agent ID 999 doesn't exist in mock
        assert_err!(
            IbcLite::register_cross_chain_agent(
                frame_system::RawOrigin::Signed(10).into(),
                b"remote-chain".to_vec(),
                b"remote-agent-1".to_vec(),
                999,
            ),
            Error::<Runtime>::AgentNotFound
        );
    });
}

// =========================================================
// M3 — open_channel_confirm tests
// =========================================================

/// Helper: open a channel in Init state (without auto-transitioning to Open).
fn open_channel_init_only(channel_num: u64) -> Vec<u8> {
    let counterparty_chain = format!("chain-{}", channel_num).into_bytes();
    let counterparty_channel = format!("remote-channel-{}", channel_num).into_bytes();

    assert_ok!(IbcLite::open_channel(
        frame_system::RawOrigin::Root.into(),
        counterparty_chain,
        counterparty_channel,
    ));

    format!("channel-{}", channel_num).into_bytes()
}

#[test]
fn open_channel_confirm_transitions_init_to_open() {
    new_test_ext().execute_with(|| {
        // Add trusted relayer
        assert_ok!(IbcLite::add_relayer(frame_system::RawOrigin::Root.into(), 10));

        let channel_id = open_channel_init_only(0);

        // Verify channel starts in Init state
        let bounded_id: ChannelId<Runtime> = channel_id.clone().try_into().unwrap();
        let channel = Channels::<Runtime>::get(&bounded_id).unwrap();
        assert_eq!(channel.state, ChannelState::Init);

        // Confirm the channel open — should transition to Open
        assert_ok!(IbcLite::open_channel_confirm(
            frame_system::RawOrigin::Signed(10).into(),
            channel_id.clone(),
        ));

        let channel = Channels::<Runtime>::get(&bounded_id).unwrap();
        assert_eq!(channel.state, ChannelState::Open, "channel must be Open after confirm");
    });
}

#[test]
fn open_channel_confirm_rejects_non_init() {
    new_test_ext().execute_with(|| {
        // Add trusted relayer
        assert_ok!(IbcLite::add_relayer(frame_system::RawOrigin::Root.into(), 10));

        let channel_id = open_channel_init_only(0);

        // First confirm: Init → Open
        assert_ok!(IbcLite::open_channel_confirm(
            frame_system::RawOrigin::Signed(10).into(),
            channel_id.clone(),
        ));

        // Second confirm on an already-Open channel must fail with InvalidChannelState
        assert_err!(
            IbcLite::open_channel_confirm(
                frame_system::RawOrigin::Signed(10).into(),
                channel_id.clone(),
            ),
            Error::<Runtime>::InvalidChannelState
        );
    });
}

#[test]
fn open_channel_confirm_requires_trusted_relayer() {
    new_test_ext().execute_with(|| {
        let channel_id = open_channel_init_only(0);

        // Unsigned-ish but signed by account 99 who is NOT a relayer
        assert_err!(
            IbcLite::open_channel_confirm(
                frame_system::RawOrigin::Signed(99).into(),
                channel_id,
            ),
            Error::<Runtime>::NotTrustedRelayer
        );
    });
}

#[test]
fn open_channel_confirm_allows_send_packet_after_confirm() {
    new_test_ext().execute_with(|| {
        // Add trusted relayer
        assert_ok!(IbcLite::add_relayer(frame_system::RawOrigin::Root.into(), 10));

        let channel_id = open_channel_init_only(0);

        // Confirm open
        assert_ok!(IbcLite::open_channel_confirm(
            frame_system::RawOrigin::Signed(10).into(),
            channel_id.clone(),
        ));

        // Now send_packet should succeed (channel is Open)
        let payload = crate::types::PacketPayload::<Runtime>::Raw(
            frame_support::BoundedVec::try_from(b"hello".to_vec()).unwrap(),
        );
        assert_ok!(IbcLite::send_packet(
            frame_system::RawOrigin::Signed(1).into(),
            channel_id,
            b"chain-0".to_vec(),
            b"remote-channel-0".to_vec(),
            None,
            payload,
        ));
    });
}
