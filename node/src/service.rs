//! ClawChain service implementation. Assembles the node components.
//!
//! This module handles the creation and configuration of the Substrate node
//! service, including block import, consensus (Aura + GRANDPA voter), and
//! networking.
//!
//! # GRANDPA Voter
//!
//! ClawChain runs a **full GRANDPA voter** (not observer mode) to achieve
//! Byzantine-fault-tolerant finality.  The voter is wired through the
//! GRANDPA block-import wrapper so that imported blocks receive justification
//! proofs, and the Aura block-authoring pipeline produces blocks that the
//! voter can then finalise.
//!
//! Architecture:
//! ```text
//! Block authoring (Aura)
//!         │
//!         ▼
//! GrandpaBlockImport  ──→  LongestChain select
//!         │
//!         ▼
//! FullClient (storage / state)
//!         │
//!         ▼
//! GRANDPA voter (run_grandpa_voter) ── gossip ──→ peers
//! ```

use clawchain_runtime::{self, opaque::Block, RuntimeApi};
use sc_consensus_aura::{ImportQueueParams, SlotProportion, StartAuraParams};
use sc_consensus_grandpa::SharedVoterState;
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use std::{sync::Arc, time::Duration};

/// How often GRANDPA generates a justification (in blocks).
///
/// Smaller values produce more frequent justifications at the cost of slightly
/// higher bandwidth; larger values reduce overhead but increase the time
/// between on-chain proofs of finality.  512 blocks ≈ 51 minutes at 6-second
/// slot times.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

// ─── Type aliases ────────────────────────────────────────────────────────────

/// The full client type definition.
pub type FullClient = sc_service::TFullClient<
    Block,
    RuntimeApi,
    sc_executor::WasmExecutor<sp_io::SubstrateHostFunctions>,
>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// The extra components returned by [`new_partial`].
///
/// Tuple layout: `(telemetry, grandpa_link, grandpa_block_import)`
type PartialOther = (
    Option<Telemetry>,
    sc_consensus_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
    sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
);

// ─── new_partial ─────────────────────────────────────────────────────────────

/// Creates the partial components needed to build the full node.
///
/// The returned [`PartialComponents`] carries the GRANDPA link and block-import
/// wrapper in its `other` field so that [`new_full`] can start the voter and
/// wire Aura's block-import correctly.
pub fn new_partial(
    config: &Configuration,
) -> Result<
    sc_service::PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        sc_consensus::DefaultImportQueue<Block>,
        sc_transaction_pool::TransactionPoolHandle<Block, FullClient>,
        PartialOther,
    >,
    ServiceError,
> {
    // ── Telemetry ──────────────────────────────────────────────────────────────
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    // ── Executor + client ──────────────────────────────────────────────────────
    let executor = sc_service::new_wasm_executor::<sp_io::SubstrateHostFunctions>(&config.executor);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager
            .spawn_handle()
            .spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    // ── Transaction pool ───────────────────────────────────────────────────────
    let transaction_pool = sc_transaction_pool::Builder::new(
        task_manager.spawn_essential_handle(),
        client.clone(),
        config.role.is_authority().into(),
    )
    .with_options(config.transaction_pool.clone())
    .with_prometheus(config.prometheus_registry())
    .build();

    // ── GRANDPA block import ───────────────────────────────────────────────────
    //
    // The GRANDPA block-import wrapper intercepts imported blocks so that it
    // can track justifications and signal the voter when the chain is ready to
    // vote.  It must be created before the Aura import queue because the queue
    // routes blocks *through* it.
    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        GRANDPA_JUSTIFICATION_PERIOD,
        &(client.clone() as Arc<_>),
        select_chain.clone(),
        telemetry.as_ref().map(|x: &Telemetry| x.handle()),
    )?;

    // ── Aura import queue ──────────────────────────────────────────────────────
    //
    // `block_import` is the GRANDPA wrapper (not the raw client) so that all
    // imported blocks go through GRANDPA-aware processing.
    // `justification_import` routes GRANDPA justifications back to the wrapper.
    let import_queue = sc_consensus_aura::import_queue::<AuraPair, _, _, _, _, _>(
        ImportQueueParams {
            block_import: grandpa_block_import.clone(),
            justification_import: Some(Box::new(grandpa_block_import.clone())),
            client: client.clone(),
            create_inherent_data_providers: move |_, ()| async move {
                let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                let slot =
                    sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                        *timestamp,
                        sp_consensus_aura::SlotDuration::from_millis(
                            clawchain_runtime::SLOT_DURATION,
                        ),
                    );
                Ok((slot, timestamp))
            },
            spawner: &task_manager.spawn_essential_handle(),
            registry: config.prometheus_registry(),
            check_for_equivocation: Default::default(),
            telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
            compatibility_mode: Default::default(),
        },
    )?;

    Ok(sc_service::PartialComponents {
        client,
        backend,
        task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool: transaction_pool.into(),
        other: (telemetry, grandpa_link, grandpa_block_import),
    })
}

// ─── new_full ─────────────────────────────────────────────────────────────────

/// Build and run the full ClawChain node.
///
/// Starts Aura block authoring (when the node is an authority) and the full
/// GRANDPA voter (unless `--no-grandpa` is passed on the CLI).
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
    let sc_service::PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain,
        transaction_pool,
        other: (mut telemetry, grandpa_link, grandpa_block_import),
    } = new_partial(&config)?;

    // ── GRANDPA protocol name ──────────────────────────────────────────────────
    //
    // The protocol name is derived from the genesis block hash so that nodes on
    // different chains (mainnet vs testnet) cannot accidentally peer with each
    // other over the GRANDPA gossip channel.
    let genesis_hash = client
        .hash(0u32.into())
        .ok()
        .flatten()
        .expect("Genesis block always exists; qed");
    let grandpa_protocol_name =
        sc_consensus_grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);

    // ── Network configuration ──────────────────────────────────────────────────
    let mut net_config = sc_network::config::FullNetworkConfiguration::<
        Block,
        <Block as sp_runtime::traits::Block>::Hash,
        sc_network::NetworkWorker<Block, <Block as sp_runtime::traits::Block>::Hash>,
    >::new(&config.network, config.prometheus_registry().cloned());

    // Notification metrics are shared between the GRANDPA gossip sub-protocol
    // and the main network worker.
    let notification_metrics = sc_network::NotificationMetrics::new(config.prometheus_registry());

    // Register the GRANDPA notification protocol so that peers can exchange
    // votes and justifications.  `grandpa_peers_set_config` now requires the
    // peer-store handle and notification metrics in addition to the protocol
    // name, and returns the notification service that must be forwarded to the
    // GRANDPA voter via `GrandpaParams::notification_service`.
    let (grandpa_peers_config, grandpa_notification_service) =
        sc_consensus_grandpa::grandpa_peers_set_config::<
            Block,
            sc_network::NetworkWorker<Block, <Block as sp_runtime::traits::Block>::Hash>,
        >(
            grandpa_protocol_name.clone(),
            notification_metrics.clone(),
            net_config.peer_store_handle(),
        );
    net_config.add_notification_protocol(grandpa_peers_config);

    let (network, system_rpc_tx, tx_handler_controller, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_config: None,
            block_relay: None,
            metrics: notification_metrics,
        })?;

    // ── Extract config fields before it is consumed by spawn_tasks ────────────
    let role = config.role;
    let force_authoring = config.force_authoring;
    let name = config.network.node_name.clone();
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();

    // ── RPC builder ────────────────────────────────────────────────────────────
    let rpc_extensions_builder = {
        let client = client.clone();
        let pool = transaction_pool.clone();

        Box::new(move |_| {
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
            };
            crate::rpc::create_full(deps).map_err(Into::into)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        network: network.clone(),
        client: client.clone(),
        keystore: keystore_container.keystore(),
        task_manager: &mut task_manager,
        transaction_pool: transaction_pool.clone(),
        rpc_builder: rpc_extensions_builder,
        backend: backend.clone(),
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        config,
        telemetry: telemetry.as_mut(),
        tracing_execute_block: None,
    })?;

    // ── Aura block authoring ───────────────────────────────────────────────────
    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x: &Telemetry| x.handle()),
        );

        let slot_duration = sc_consensus_aura::slot_duration(&*client)?;

        // Aura uses the GRANDPA block-import wrapper as its block_import so
        // that authored blocks flow through the GRANDPA pipeline immediately.
        let aura = sc_consensus_aura::start_aura::<AuraPair, _, _, _, _, _, _, _, _, _, _>(
            StartAuraParams {
                slot_duration,
                client: client.clone(),
                select_chain,
                block_import: grandpa_block_import,
                proposer_factory,
                create_inherent_data_providers: move |_, ()| async move {
                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
                    let slot =
                        sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            sp_consensus_aura::SlotDuration::from_millis(
                                clawchain_runtime::SLOT_DURATION,
                            ),
                        );
                    Ok((slot, timestamp))
                },
                force_authoring,
                backoff_authoring_blocks: Option::<()>::None,
                keystore: keystore_container.keystore(),
                sync_oracle: sync_service.clone(),
                justification_sync_link: sync_service.clone(),
                block_proposal_slot_portion: SlotProportion::new(2f32 / 3f32),
                max_block_proposal_slot_portion: None,
                telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
                compatibility_mode: Default::default(),
            },
        )?;

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("aura", Some("block-authoring"), aura);
    }

    // ── GRANDPA voter ──────────────────────────────────────────────────────────
    //
    // Run a full GRANDPA voter on all non-light nodes.  The voter gossips
    // prevote / precommit messages with peers and produces justifications that
    // prove finality of blocks.
    //
    // Authority nodes supply a keystore so the voter can sign votes; non-
    // authority (full-node) peers participate in gossip but do not sign.
    if enable_grandpa {
        let grandpa_config = sc_consensus_grandpa::Config {
            // Target gossip period — shorter = lower latency, higher bandwidth.
            gossip_duration: Duration::from_millis(333),
            justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
            name: Some(name),
            // Enable signing only on authority nodes.
            observer_enabled: false,
            keystore: if role.is_authority() {
                Some(keystore_container.keystore())
            } else {
                None
            },
            local_role: role,
            telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
            protocol_name: grandpa_protocol_name,
        };

        let grandpa_voter =
            sc_consensus_grandpa::run_grandpa_voter(sc_consensus_grandpa::GrandpaParams {
                config: grandpa_config,
                link: grandpa_link,
                network,
                sync: sync_service,
                notification_service: grandpa_notification_service,
                voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
                prometheus_registry,
                shared_voter_state: SharedVoterState::empty(),
                telemetry: telemetry.as_ref().map(|x: &Telemetry| x.handle()),
                offchain_tx_pool_factory:
                    sc_transaction_pool_api::OffchainTransactionPoolFactory::new(transaction_pool),
            })?;

        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", None, grandpa_voter);

        log::info!("GRANDPA voter started");
    } else {
        log::info!("GRANDPA disabled via --no-grandpa flag");
    }

    Ok(task_manager)
}
