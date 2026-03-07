# Execution Plan: [Task Name]

## Status: [Planning | In Progress | Review | Complete]
## Complexity: [Low | Medium | High]
## Pallets affected: [list]

---

## Goal

[One paragraph — what does done look like? Be concrete. Include: what feature/fix is delivered,
how it will be tested, and what CI gates will be green.]

---

## Steps

- [ ] Step 1: Read docs/ARCHITECTURE.md for relevant dependency rules
- [ ] Step 2: [describe implementation step]
- [ ] Step 3: [describe implementation step]
- [ ] Step 4: Write tests covering happy path + all error variants
- [ ] Step 5: Add/update benchmarks if new extrinsics added
- [ ] Step 6: Run `cargo test --workspace` — must pass
- [ ] Step 7: Run `bash scripts/agent-lint.sh` — must pass
- [ ] Step 8: Run `cargo clippy --workspace -- -D warnings` — must pass
- [ ] Step 9: Update relevant docs/ if architecture changed
- [ ] Step 10: Open PR with this plan linked in description

---

## Decisions

| Decision | Rationale | Date |
|----------|-----------|------|
| | | |

---

## Known Risks

- [List risks that could block completion or require architectural changes]

---

## Cross-Pallet Impact

If this task touches the Config trait boundary or adds a new cross-pallet dependency:
1. Check docs/ARCHITECTURE.md for existing dependency graph
2. Confirm the new dependency does not create a cycle
3. Update docs/ARCHITECTURE.md with the new dependency
4. Add the trait definition to the *provider* pallet's crate, not the consumer's

---

## Notes for Reviewer

[What should the reviewer focus on? Any tricky logic? Any deviations from convention?]

---

*Copy this template to `docs/plans/<task-slug>.md` before starting complex work.*
*Do not start coding until Steps 1+ are filled in.*
