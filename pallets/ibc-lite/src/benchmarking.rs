//! IBC-lite pallet benchmarks.
//!
//! These are stub benchmarks. Real benchmarks should be implemented
//! using the `runtime-benchmarks` feature and FRAME benchmarking framework.

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    /// Benchmark for opening a new channel.
    #[benchmark]
    fn open_channel() {
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();

        #[extrinsic_call]
        _(RawOrigin::Root, counterparty_chain_id, counterparty_channel_id);
    }

    /// Benchmark for initiating channel closure.
    #[benchmark]
    fn close_channel_init() {
        // Setup: create a channel first
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();
        Pallet::<T>::open_channel(
            RawOrigin::Root.into(),
            counterparty_chain_id,
            counterparty_channel_id,
        )?;

        let channel_id = b"channel-0".to_vec();

        #[extrinsic_call]
        _(RawOrigin::Root, channel_id);
    }

    /// Benchmark for confirming channel closure.
    #[benchmark]
    fn close_channel_confirm() {
        // Setup: create and init-close a channel
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();
        Pallet::<T>::open_channel(
            RawOrigin::Root.into(),
            counterparty_chain_id,
            counterparty_channel_id,
        )?;

        let channel_id = b"channel-0".to_vec();
        Pallet::<T>::close_channel_init(RawOrigin::Root.into(), channel_id.clone())?;

        // Add relayer
        let relayer: T::AccountId = account("relayer", 0, 0);
        Pallet::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone())?;

        #[extrinsic_call]
        _(RawOrigin::Signed(relayer), channel_id);
    }

    /// Benchmark for sending a packet.
    #[benchmark]
    fn send_packet() {
        // Setup: create a channel
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();
        Pallet::<T>::open_channel(
            RawOrigin::Root.into(),
            counterparty_chain_id,
            counterparty_channel_id,
        )?;

        let channel_id = b"channel-0".to_vec();
        let dst_chain_id = b"benchmark-chain".to_vec();
        let dst_channel_id = b"benchmark-channel-0".to_vec();

        // Create a max-sized payload
        let mut payload_bytes = vec![0u8; T::MaxPayloadLen::get() as usize];
        payload_bytes[0] = 1;
        let payload = PacketPayload::<T>::Raw(
            payload_bytes
                .try_into()
                .unwrap_or_else(|_| BoundedVec::default()),
        );

        #[extrinsic_call]
        _(
            RawOrigin::Signed(account("caller", 0, 0)),
            channel_id,
            dst_chain_id,
            dst_channel_id,
            None,
            payload,
        );
    }

    /// Benchmark for receiving a packet.
    #[benchmark]
    fn receive_packet() {
        // Setup: create a channel
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();
        Pallet::<T>::open_channel(
            RawOrigin::Root.into(),
            counterparty_chain_id,
            counterparty_channel_id,
        )?;

        let channel_id: ChannelId<T> = b"channel-0".to_vec().try_into().unwrap();
        let remote_channel_id: ChannelId<T> = b"benchmark-channel-0".to_vec().try_into().unwrap();
        let chain_id: ChainId<T> = b"benchmark-chain".to_vec().try_into().unwrap();

        // Create a max-sized payload
        let mut payload_bytes = vec![0u8; T::MaxPayloadLen::get() as usize];
        payload_bytes[0] = 1;
        let payload = PacketPayload::<T>::Raw(
            payload_bytes
                .try_into()
                .unwrap_or_else(|_| BoundedVec::default()),
        );

        let packet = Packet::<T> {
            sequence: 1,
            src_channel_id: remote_channel_id,
            dst_channel_id: channel_id,
            dst_chain_id: chain_id,
            src_agent_id: None,
            dst_agent_id: None,
            payload,
            timeout_height: 10000u32.into(),
            created_at: 100u32.into(),
        };

        // Add relayer
        let relayer: T::AccountId = account("relayer", 0, 0);
        Pallet::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone())?;

        #[extrinsic_call]
        _(RawOrigin::Signed(relayer), packet);
    }

    /// Benchmark for acknowledging a packet.
    #[benchmark]
    fn acknowledge_packet() {
        // Setup: create a channel and send a packet
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();
        Pallet::<T>::open_channel(
            RawOrigin::Root.into(),
            counterparty_chain_id,
            counterparty_channel_id,
        )?;

        let channel_id = b"channel-0".to_vec();
        let caller: T::AccountId = account("caller", 0, 0);

        let mut payload_bytes = vec![0u8; T::MaxPayloadLen::get() as usize];
        payload_bytes[0] = 1;
        let payload = PacketPayload::<T>::Raw(
            payload_bytes
                .try_into()
                .unwrap_or_else(|_| BoundedVec::default()),
        );

        Pallet::<T>::send_packet(
            RawOrigin::Signed(caller).into(),
            channel_id.clone(),
            b"benchmark-chain".to_vec(),
            b"benchmark-channel-0".to_vec(),
            None,
            payload,
        )?;

        // Create ack payload
        let ack = PacketPayload::<T>::Ack {
            success: true,
            error_code: None,
            data: BoundedVec::default(),
        };

        // Add relayer
        let relayer: T::AccountId = account("relayer", 0, 0);
        Pallet::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone())?;

        #[extrinsic_call]
        _(RawOrigin::Signed(relayer), channel_id, 1u64, ack);
    }

    /// Benchmark for timing out a packet.
    #[benchmark]
    fn timeout_packet() {
        // Setup: create a channel and send a packet
        let counterparty_chain_id = b"benchmark-chain".to_vec();
        let counterparty_channel_id = b"benchmark-channel-0".to_vec();
        Pallet::<T>::open_channel(
            RawOrigin::Root.into(),
            counterparty_chain_id,
            counterparty_channel_id,
        )?;

        let channel_id = b"channel-0".to_vec();
        let caller: T::AccountId = account("caller", 0, 0);

        let payload = PacketPayload::<T>::Raw(BoundedVec::default());

        Pallet::<T>::send_packet(
            RawOrigin::Signed(caller).into(),
            channel_id.clone(),
            b"benchmark-chain".to_vec(),
            b"benchmark-channel-0".to_vec(),
            None,
            payload,
        )?;

        #[extrinsic_call]
        _(RawOrigin::Signed(account("caller", 0, 0)), channel_id, 1u64);
    }

    /// Benchmark for adding a relayer.
    #[benchmark]
    fn add_relayer() {
        let relayer: T::AccountId = account("relayer", 0, 0);

        #[extrinsic_call]
        _(RawOrigin::Root, relayer);
    }

    /// Benchmark for removing a relayer.
    #[benchmark]
    fn remove_relayer() {
        // Setup: add a relayer first
        let relayer: T::AccountId = account("relayer", 0, 0);
        Pallet::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone())?;

        #[extrinsic_call]
        _(RawOrigin::Root, relayer);
    }

    /// Benchmark for registering a cross-chain agent.
    #[benchmark]
    fn register_cross_chain_agent() {
        // Setup: add a relayer
        let relayer: T::AccountId = account("relayer", 0, 0);
        Pallet::<T>::add_relayer(RawOrigin::Root.into(), relayer.clone())?;

        let chain_id = b"remote-chain".to_vec();
        let remote_agent_id = b"remote-agent-1".to_vec();
        let local_agent_id = 1u64;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(relayer),
            chain_id,
            remote_agent_id,
            local_agent_id,
        );
    }

    impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}
