use anki::sync::http_server::{default_ip_header, SimpleServer, SyncServerConfig};
use anki::sync::login::sync_login;
use ankimarkable_pure::backend::open_collection;
use ankimarkable_pure::backend::{Backend, Grade};
use ankimarkable_pure::demo;

#[test]
fn self_hosted_sync_downloads_decks_and_uploads_reviews() {
    let directory = std::env::temp_dir().join(format!("pure-sync-{}", std::process::id()));
    let seed = directory.join("seed.anki2");
    let target = directory.join("tablet.anki2");
    demo::create(&seed).unwrap();
    std::env::set_var("SYNC_USER1", "demo@example.test:test-password");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (address, server) = runtime
        .block_on(SimpleServer::make_server(SyncServerConfig {
            host: "127.0.0.1".parse().unwrap(),
            port: 0,
            base_folder: directory.join("server"),
            ip_header: default_ip_header(),
        }))
        .unwrap();
    let task = runtime.spawn(server);
    let endpoint = format!("http://{address}/");
    std::env::set_var("ANKIMARKABLE_SYNC_ENDPOINT", &endpoint);
    let client = reqwest::Client::new();
    let mut auth = runtime
        .block_on(sync_login(
            "demo@example.test",
            "test-password",
            Some(endpoint.clone()),
            client.clone(),
        ))
        .unwrap();
    auth.endpoint = Some(endpoint.parse().unwrap());
    let collection = open_collection(&seed).unwrap();
    runtime
        .block_on(collection.full_upload(auth.clone(), client.clone()))
        .unwrap();
    let mut backend = Backend::open(seed.to_str().unwrap()).unwrap();
    backend.sync("demo@example.test", "test-password").unwrap();
    drop(backend);
    demo::create(&target).unwrap();
    let connection = rusqlite::Connection::open(&target).unwrap();
    connection
        .execute("update col set scm=scm+100000", [])
        .unwrap();
    drop(connection);
    let mut tablet = Backend::open(target.to_str().unwrap()).unwrap();
    let account =
        Backend::login("demo@example.test", "test-password", Some(endpoint.clone())).unwrap();
    let saved = serde_json::to_string(&account).unwrap();
    assert!(!saved.contains("test-password"));
    let account = serde_json::from_str(&saved).unwrap();
    tablet.sync_account(&account).unwrap();
    let backups: Vec<_> = std::fs::read_dir(directory.join(".sync-backups"))
        .unwrap()
        .collect();
    assert_eq!(
        backups.len(),
        1,
        "Full download must preserve the previous local collection"
    );
    tablet.set_deck_scope(demo::DECK_NAME).unwrap();
    assert_eq!(tablet.counts().unwrap().new, 5);
    assert!(target
        .with_extension("media")
        .join("demo-grid.png")
        .is_file());
    let card = tablet.next_card().unwrap().unwrap();
    tablet.answer(&card, Grade::Good).unwrap();
    tablet.sync_account(&account).unwrap();
    drop(tablet);
    let mut desktop = Backend::open(seed.to_str().unwrap()).unwrap();
    desktop.sync("demo@example.test", "test-password").unwrap();
    drop(desktop);
    let connection = rusqlite::Connection::open(&seed).unwrap();
    let reviews: i64 = connection
        .query_row("SELECT count(*) FROM revlog", [], |row| row.get(0))
        .unwrap();
    assert_eq!(reviews, 1);
    drop(connection);
    task.abort();
    drop(runtime);
    std::env::remove_var("ANKIMARKABLE_SYNC_ENDPOINT");
    std::env::remove_var("SYNC_USER1");
    std::fs::remove_dir_all(directory).unwrap();
}
