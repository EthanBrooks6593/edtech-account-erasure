use edtech_account_erasure::{
    infrai_control::InfraiClient,
    learner_erasure::{erase_account, LearningRecords},
};
use std::{env, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let user_id = match args.next() {
        Some(value) => value,
        None => return usage(),
    };
    let credential_id = match args.next() {
        Some(value) => value,
        None => return usage(),
    };
    if args.next().is_some() {
        return usage();
    }

    let client = match InfraiClient::from_env() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("configuration error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut records = LearningRecords::default();

    match erase_account(&client, &mut records, &user_id, &credential_id).await {
        Ok(receipt) => {
            println!(
                "erased user={} sessions={} deliveries={} deadlines={} report_rows={} credential_revoked={}",
                receipt.user_id,
                receipt.sessions_revoked,
                receipt.deliveries_removed,
                receipt.deadlines_removed,
                receipt.report_rows_removed,
                receipt.credential_revoked
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("account erasure rejected: {error}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: erase_learner <user_id> <credential_id>");
    ExitCode::FAILURE
}
