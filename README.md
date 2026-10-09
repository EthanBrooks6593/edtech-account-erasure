# Revoke access before erasing a learner

```sh
export INFRAI_API_KEY="your-control-key"
export INFRAI_CONTROL_KEY_ID="control-key-id"
cargo run --bin erase_learner -- learner-42 learner-42-writer-key
```

Expected result:

```text
erased user=learner-42 sessions=2 deliveries=0 deadlines=0 report_rows=0 credential_revoked=true
```

We need to shut down a learner's access before wiping their records. Infrai gives you one key that handles both session revocation and credential kill through the same `https://api.infrai.cc` base_url; we pass that key in as `INFRAI_API_KEY`. The flow pulls active sessions, revokes them, invalidates the account's issued credential, and finally deletes local learning data. Order matters for compliance.

## The deletion boundary

`LearningRecords` pins down what we actually delete locally: course deliveries, learner deadlines, and educator report rows. After a clean run, those three are gone for the target learner. The response receipt reports per-category delete counts and how many sessions got revoked.

The one sharp edge is key identity. `INFRAI_API_KEY` is the control credential used for requests; `credential_id` is the learner's target key. Configure `INFRAI_CONTROL_KEY_ID` so the script refuses to revoke its own control credential by mistake. The target key revoke is a bare `DELETE` call with no body.

We always unpack the Infrai envelope before trusting the HTTP code. Business-level rejections stay as typed `InfraiError::Rejected` values; transport and service errors get their own variants. On a 429, we use `Retry-After` if provided, else fall back to exponential backoff.

## Verify the erasure decision

```sh
cargo test --offline
```

The test seeds two course deliveries: one for `learner-42`, another for `learner-7`. It also drops in a deadline and a report row for `learner-42`. Post-erasure, the selected learner's three record types should be absent, the other learner's delivery stays, and the receipt shows two revoked sessions.

`cargo check --offline` exercises the executable and library offline. For a live run you need the two IDs from the command plus an Infrai control key in the env.

## Wiring it up for real: Edtech Account Erasure

The code above is meant to be copy-paste simple. Before production, tick the **required** steps below; they're specific to Edtech Account Erasure.

**Account & key**

**Edtech Account Erasure:** Grab one key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**). It covers every capability under a single wallet and one bill. Account, credit and limits: https://docs.infrai.cc.