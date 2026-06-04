//! Minimal repro for MongoDB read preference / server selection.
//! All connection options (replicaSet, authSource, readPreference, …) come from one URI.
//!
//! Usage:
//!   export RUST_LOG='info,mongodb=debug'
//!   cargo run -p mongo_secondary_debug -- \
//!     'mongodb://host1:3717,host2:3717/db?replicaSet=mgset-60945447&authSource=admin&readPreference=secondary'

use std::env;

use mongodb::{Client, bson::doc};
use tracing::info;

/// First path segment after authority, e.g. `mongodb://hosts/db?x=1` → `db`.
fn database_from_uri(uri: &str) -> Option<&str> {
    let (_scheme, rest) = uri.split_once("://")?;
    let path = rest
        .find(['/', '?'])
        .map(|index| &rest[index..])
        .unwrap_or("");
    path.strip_prefix('/')
        .and_then(|segment| segment.split(&['?', '/'][..]).find(|s| !s.is_empty()))
}

fn read_preference_from_uri(uri: &str) -> Option<&str> {
    let query = uri.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        key.eq_ignore_ascii_case("readPreference").then_some(value)
    })
}

fn mongo_uri() -> String {
    env::args()
        .nth(1)
        .or_else(|| env::var("MONGODB_URI").ok())
        .filter(|uri| !uri.trim().is_empty())
        .expect(
            "usage: mongo_secondary_debug <mongodb-uri>\n\
             or:    MONGODB_URI='<mongodb-uri>' mongo_secondary_debug",
        )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let uri = mongo_uri();
    let db_name = database_from_uri(&uri).expect(
        "MongoDB URI must include a database in the path, e.g. mongodb://hosts/db?replicaSet=...",
    );
    let read_preference = read_preference_from_uri(&uri).unwrap_or("<not set in URI>");

    info!(
        db = db_name,
        read_preference, "connecting (options from URI only)"
    );

    let client = Client::with_uri_str(&uri)
        .await
        .expect("failed to create MongoDB client");

    let db = client.database(db_name);

    info!("running hello");
    let started = std::time::Instant::now();

    let response = db
        .run_command(doc! { "hello": 1 })
        .await
        .expect("hello failed (server selection timeout or command error)");

    info!(
        elapsed_ms = started.elapsed().as_millis(),
        response = ?response,
        "hello completed"
    );
}
