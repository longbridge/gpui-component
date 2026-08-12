# Architecture Specifications

- [`RFC.md`](RFC.md) is the authoritative Open-Code architecture and acceptance specification.
- [`PHASE1.md`](PHASE1.md) records the original architecture direction and phased migration proposal.
- [`BASE-TODO.md`](BASE-TODO.md) tracks milestones, current state, decisions, risks, and verification evidence.
- [`BASE-REVIEW.md`](BASE-REVIEW.md) reviews the `crates/base` component surface and
  proposes the adjustment plan: abstraction taxonomy, cross-cutting inconsistencies,
  per-issue remediation with affected files, upstream GPUI requests, and batch order.
- [`STYLE-MODIFIER.md`](STYLE-MODIFIER.md) proposes typed semantic-state styling and
  application-owned motion primitives; it remains a revised draft until its
  milestones are accepted into `RFC.md`.

When the documents conflict, follow `RFC.md`. Update `BASE-TODO.md` as work is
implemented and verified.
