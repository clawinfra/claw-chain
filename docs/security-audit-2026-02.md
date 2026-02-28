
---

## Audit Batch 2 — 2026-02-28: pallet-ibc-lite, pallet-anon-messaging, pallet-service-market

**Auditor:** Alex Chen (ClawInfra AI agent)
**Date:** 2026-02-28
**Scope:** 3 pallets — ibc-lite, anon-messaging, service-market
**Total findings:** 1 CRITICAL · 4 HIGH · 3 MEDIUM · 3 LOW

All CRITICAL and HIGH findings fixed in this PR. MEDIUM findings annotated with TODO comments.

---

### CRITICAL

#### C1 — `pallet-ibc-lite`: `timeout_packet` missing timeout height check
**File:** `pallets/ibc-lite/src/lib.rs` — `timeout_packet` extrinsic
**Severity:** CRITICAL
**Description:** The `timeout_packet` extrinsic accepted any signed caller and had no check that the packet's timeout height had actually elapsed. Comment in code read "For now, we assume the caller has verified." Any user could call this on any pending packet commitment at any time, permanently destroying valid in-flight cross-chain packets and causing message loss.
**Fix:** Added `PacketTimeoutHeights` StorageDoubleMap. `send_packet` now stores the `timeout_height` at send time. `timeout_packet` reads and validates `now >= timeout_height` before removing the commitment. Added regression test `timeout_packet_rejected_before_timeout`.

---

### HIGH

#### H1 — `pallet-ibc-lite`: Raw `*seq += 1` overflow in `acknowledge_packet`
**File:** `pallets/ibc-lite/src/lib.rs` — `acknowledge_packet` extrinsic
**Severity:** HIGH
**Description:** `AckSequences` counter was incremented with raw `*seq += 1`. On u64 overflow (wrap on debug, panic in release) this would corrupt the ack sequence tracking and break packet ordering.
**Fix:** Changed to `seq.saturating_add(1)`.

#### H2 — `pallet-service-market`: Multiple raw arithmetic operations on counters and deadlines
**File:** `pallets/service-market/src/lib.rs`
**Severity:** HIGH
**Description:** Four raw `+` / `+ 1` operations:
- `ListingCount`: `listing_id + 1`
- `InvocationCount`: `invocation_id + 1`
- `DisputeCount`: `dispute_id + 1`
- Deadline: `now + deadline_blocks.into()`

On near-max u64 counters or block numbers, these would panic (debug) or wrap (release), corrupting storage.
**Fix:** All changed to `.saturating_add(1)` / `.saturating_add(deadline_blocks.into())`. Added `use sp_runtime::traits::Saturating` import.

#### H3 — `pallet-service-market`: `resolve_dispute_governance` winner not validated as party
**File:** `pallets/service-market/src/lib.rs` — `resolve_dispute_governance` extrinsic
**Severity:** HIGH
**Description:** The `winner` parameter was not checked to be the invoker or provider of the invocation. Governance (sudo) could award escrow to any arbitrary account including one with no relation to the dispute.
**Fix:** Added `ensure!(inv.invoker == winner || inv.provider == winner, Error::<T>::NotPartyToInvocation)` before updating the dispute record.

#### H4 — `pallet-service-market`: `on_initialize` full storage iteration DoS
**File:** `pallets/service-market/src/lib.rs` — `expire_overdue_invocations`
**Severity:** HIGH
**Description:** `expire_overdue_invocations` used `InvocationsByDeadline::<T>::iter()` — a full scan of all invocation deadline entries on every block, filtered by `deadline < n`. With many invocations this is O(total_invocations) per block, a block-production DoS vector.
**Fix:** Replaced with `InvocationsByDeadline::<T>::iter_prefix(n)` which only reads entries keyed to the current block — O(items_at_block). In production, `on_initialize` is called every block so no deadlines are missed. Test updated to call `on_initialize` at the exact deadline block.

---

### MEDIUM

#### M1 — `pallet-anon-messaging`: Auto-reply cooldown not enforced
**File:** `pallets/anon-messaging/src/lib.rs` — `maybe_trigger_auto_response`
**Severity:** MEDIUM
**Description:** `AutoReplyCooldown` storage exists and tracks last auto-reply block per (responder, requester) pair, but the cooldown is never checked. An attacker can spam auto-response events regardless of configured cooldown.
**Fix:** Added TODO comment referencing this issue. Full fix requires passing the `sender` account into `maybe_trigger_auto_response` and enforcing cooldown before emitting the event. Planned for Phase 2.

#### M2 — `pallet-service-market`: `RequirementsEmpty` error never used
**File:** `pallets/service-market/src/lib.rs` — `invoke_service`
**Severity:** MEDIUM
**Description:** `Error::RequirementsEmpty` was defined but `invoke_service` never validated that requirements were non-empty. Providers could receive invocations with no requirements.
**Fix:** Added `ensure!(!requirements.is_empty(), Error::<T>::RequirementsEmpty)` before length-bounding.

#### M3 — `pallet-ibc-lite`: No `open_channel_confirm` transition
**File:** `pallets/ibc-lite/src/lib.rs`
**Severity:** MEDIUM
**Description:** Channels are opened in `ChannelState::Init` but no extrinsic exists to transition to `ChannelState::Open`. Consequently `send_packet` and `receive_packet` always fail with `ChannelNotOpen`. The pallet is currently non-functional for actual message passing.
**Note:** This appears to be a known Phase 1 limitation. A `confirm_channel_open` extrinsic (callable by trusted relayer) is required. Tracked for Phase 2 implementation.

---

### LOW

#### L1 — `pallet-anon-messaging`: No rate limit on `register_public_key`
Anyone can repeatedly update their public key with no fee. In practice this is limited by transaction fees but no explicit rate limiting exists.

#### L2 — `pallet-service-market`: No storage deposit for listings
Service listings accumulate unbounded on-chain storage with no deposit mechanism. Consider `T::Currency::reserve` per listing for storage spam prevention.

#### L3 — `pallet-anon-messaging`: Sender-side message delete not implemented
`delete_message` only works when the caller is the receiver (it looks up `Inbox::<T>::get(&who, msg_id)`). The sender authorization check on line `ensure!(envelope.sender == who || envelope.receiver == who, ...)` is unreachable for senders. Acknowledged design gap (Phase 2: add `SentIndex`).

