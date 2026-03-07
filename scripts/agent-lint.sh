#!/bin/bash
# Agent harness linter — errors are written to be agent-readable
# Every error message includes: what it is, how to fix it, which doc to consult.
set -euo pipefail

ERRORS=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

echo "=== ClawChain Agent Lint ==="
echo "Repo: $REPO_ROOT"
echo ""

# ---------------------------------------------------------------------------
# Rule 1: All pallets must have benchmarking.rs
# ---------------------------------------------------------------------------
echo "[1/5] Checking benchmarks..."
for pallet_dir in pallets/*/; do
  pallet=$(basename "$pallet_dir")
  # Skip pallets explicitly marked as harness-exempt (existing pallets predating the harness)
  if grep -q 'harness-exempt.*benchmarks' "${pallet_dir}Cargo.toml" 2>/dev/null; then
    echo "  [skip] $pallet — benchmarks-pending (harness-exempt)"
    continue
  fi
  if [ ! -f "${pallet_dir}src/benchmarking.rs" ]; then
    echo ""
    echo "LINT ERROR [missing-benchmarks]: $pallet has no benchmarking.rs"
    echo "  WHAT: Every pallet must have a benchmarking.rs with benchmarks for all extrinsics."
    echo "        Un-benchmarked extrinsics cannot be weight-annotated and block mainnet deployment."
    echo "  FIX:  Create ${pallet_dir}src/benchmarking.rs using the template:"
    echo "        use frame_benchmarking::v2::*;"
    echo "        #[benchmarks] mod benchmarks { use super::*;"
    echo "          #[benchmark] fn <extrinsic_name>() -> Result<(), BenchmarkError> { ... }"
    echo "          impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);"
    echo "        }"
    echo "  REF:  docs/QUALITY.md#benchmarks"
    ERRORS=$((ERRORS+1))
  fi
done

# ---------------------------------------------------------------------------
# Rule 2: No Vec in storage (must use BoundedVec)
# ---------------------------------------------------------------------------
echo "[2/5] Checking for unbounded Vec in storage..."
UNBOUNDED=$(grep -rn \
  "StorageValue.*Vec<\|StorageMap.*Vec<\|StorageDoubleMap.*Vec<\|StorageNMap.*Vec<" \
  pallets/*/src/*.rs 2>/dev/null \
  | grep -v BoundedVec \
  | grep -v "^[[:space:]]*//" \
  | grep -v "//.*StorageValue" \
  || true)

if [ -n "$UNBOUNDED" ]; then
  echo ""
  echo "LINT ERROR [unbounded-storage]: Found unbounded Vec<T> in storage declarations:"
  echo "$UNBOUNDED"
  echo ""
  echo "  WHAT: Unbounded Vec<T> in storage makes weight calculation impossible and is a DoS vector."
  echo "        Weight system requires deterministic size bounds for all storage items."
  echo "  FIX:  Replace Vec<X> with BoundedVec<X, T::MaxLen>"
  echo "        Add MaxLen: Get<u32> to the pallet's Config trait"
  echo "        Add #[pallet::constant] type MaxLen: Get<u32>; to Config"
  echo "        Example: pub name: BoundedVec<u8, T::MaxNameLen>"
  echo "  REF:  docs/ARCHITECTURE.md#bounded-storage"
  ERRORS=$((ERRORS+1))
fi

# ---------------------------------------------------------------------------
# Rule 3: Extrinsics must emit events
# ---------------------------------------------------------------------------
echo "[3/5] Checking event emission..."
for pallet_lib in pallets/*/src/lib.rs; do
  pallet=$(echo "$pallet_lib" | cut -d/ -f2)
  has_extrinsics=$(grep -c "#\[pallet::call\]" "$pallet_lib" 2>/dev/null || echo 0)
  has_events=$(grep -c "deposit_event\|Self::deposit_event" "$pallet_lib" 2>/dev/null || echo 0)
  if [ "$has_extrinsics" -gt "0" ] && [ "$has_events" -eq "0" ]; then
    echo ""
    echo "LINT ERROR [missing-events]: $pallet has #[pallet::call] extrinsics but no deposit_event calls"
    echo "  WHAT: All state-changing extrinsics must emit events so off-chain indexers can track state."
    echo "        Silent state changes make SubQuery/Subsquid/SDK blind to chain activity."
    echo "  FIX:  Add Self::deposit_event(Event::YourEvent { field: value }) at the end of each extrinsic"
    echo "        Add #[pallet::event] #[pallet::generate_deposit(pub(super) fn deposit_event)]"
    echo "        to the pallet macro block"
    echo "  REF:  docs/CONVENTIONS.md#events"
    ERRORS=$((ERRORS+1))
  fi
done

# ---------------------------------------------------------------------------
# Rule 4: Security-sensitive extrinsics must have origin checks
# ---------------------------------------------------------------------------
echo "[4/5] Checking origin checks on security-sensitive extrinsics..."
for func in "update_reputation" "treasury_spend" "invoke_service"; do
  if grep -rn "pub fn $func" pallets/*/src/lib.rs >/dev/null 2>&1; then
    # Get the file(s) containing this function
    while IFS= read -r match_file; do
      # Check 8 lines after the fn declaration for an origin check
      if ! grep -A 8 "pub fn $func" "$match_file" 2>/dev/null \
          | grep -q "ensure_root\|ensure_signed\|ensure_none\|ensure_origin\|AuthorityOrigin\|GovernanceOrigin"; then
        pallet=$(echo "$match_file" | cut -d/ -f2)
        echo ""
        echo "LINT ERROR [missing-origin-check]: $func in $pallet ($match_file) has no origin validation"
        echo "  WHAT: Security-sensitive extrinsics (update_reputation, treasury_spend, invoke_service)"
        echo "        must explicitly validate caller identity to prevent unauthorized state changes."
        echo "  FIX:  Add one of the following at the start of $func:"
        echo "          let who = ensure_signed(origin)?;         // any signed account"
        echo "          ensure_root!(origin)?;                    // sudo/governance only"
        echo "          T::AuthorityOrigin::ensure_origin(origin)?; // custom privileged origin"
        echo "  REF:  docs/QUALITY.md#security-sensitive-functions"
        ERRORS=$((ERRORS+1))
      fi
    done < <(grep -rln "pub fn $func" pallets/*/src/lib.rs 2>/dev/null)
  fi
done

# ---------------------------------------------------------------------------
# Rule 5: AGENTS.md must stay under 150 lines
# ---------------------------------------------------------------------------
echo "[5/5] Checking AGENTS.md length..."
if [ -f "AGENTS.md" ]; then
  AGENTS_LINES=$(wc -l < AGENTS.md)
  if [ "$AGENTS_LINES" -gt 150 ]; then
    echo ""
    echo "LINT ERROR [agents-too-long]: AGENTS.md is $AGENTS_LINES lines (max 150)"
    echo "  WHAT: AGENTS.md is a table of contents, not a reference manual."
    echo "        Long AGENTS.md files burn agent context on navigation instead of work."
    echo "  FIX:  Move detailed content to docs/ and replace with a pointer in AGENTS.md"
    echo "        Example: '## Architecture → See docs/ARCHITECTURE.md'"
    echo "  REF:  AGENTS.md itself (table of contents philosophy)"
    ERRORS=$((ERRORS+1))
  fi
fi

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "=== Lint complete: $ERRORS error(s) ==="
if [ $ERRORS -gt 0 ]; then
  echo ""
  echo "Fix all errors above before opening a PR."
  echo "Each error includes WHAT (the problem), FIX (how to resolve), REF (which doc to read)."
  exit 1
else
  echo "All checks passed. ✓"
fi
