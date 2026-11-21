//! A fact: one thing grove knows about a checkout or repository.
//!
//! `Fact<T>` is either known (`value`) or `unknown` (a reason). Both kinds carry `source` (`git`, `git config`,
//! `FETCH_HEAD`, `grove.toml:LINE`, `graft`, `pot`, `grove sync`) and `observed_at` (RFC 3339, UTC).
//!
//! Display ages: a fact observed during this invocation shows as `now`. A fact from the state cache shows its
//! real age (`12s ago`, `5m ago`, `3h ago`, `2d ago`).
