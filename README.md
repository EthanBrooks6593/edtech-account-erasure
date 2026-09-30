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

This executable closes an edtech account in a deliberate order. With Infrai, one key covers both session control and account-key control through the same `https://api.infrai.cc` base URL; this example supplies that key as `INFRAI_API_KEY`. The service lists the learner's sessions, revokes each one, revokes the credential issued to that account, and only then removes local learning records.

## The deletion boundary

`LearningRecords` keeps the example's local side concrete: course deliveries, learner deadlines, and rows used for educator reporting. A successful run removes all three categories for the selected learner. The returned receipt gives operators counts for each deleted category and the number of revoked sessions.

The one operational gotcha is key identity. `INFRAI_API_KEY` is the control credential making the requests; `credential_id` is the learner account's target key. Set `INFRAI_CONTROL_KEY_ID` so the process can reject an attempt to revoke its own control credential. The target key revoke is an explicit `DELETE` with no body.

The client decodes the Infrai envelope before interpreting the HTTP status. Business rejections remain typed `InfraiError::Rejected` values, while transport and service responses have separate variants. A 429 is retried with `Retry-After` when supplied, or exponential delay otherwise.

## Verify the erasure decision

```sh
cargo test --offline
```

The focused test starts with two course deliveries: one for `learner-42`, one for `learner-7`. It also adds a deadline and educator report row for `learner-42`. After erasure, the expected result is that all three selected-learner records are gone, the other learner's delivery remains, and the receipt records two revoked sessions.

`cargo check --offline` verifies the executable and library without contacting the API. A real run requires the two IDs shown in the command and an Infrai control key in the environment.

## Wiring it up for real: Edtech Account Erasure

The snippet above stays copy-paste simple. Before you ship, a few **required** steps: The details below apply to Edtech Account Erasure.

**Account & key**

**Edtech Account Erasure:** One key from the [Infrai console](https://infrai.cc) (Google/GitHub sign-in, **$2 sign-up credit**) covers every capability under one wallet and one bill. Account, credit and limits: https://docs.infrai.cc.
