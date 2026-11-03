use ankimarkable_pure::session::{Request, Session, State};
use ankimarkable_pure::transport;
use anyhow::{Context, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let socket = std::env::args()
        .nth(1)
        .context("Start Anki through AppLoad")?;
    let directory = std::env::var_os("ANKIMARKABLE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/root/.ankimarkable-pure"));
    let mut session = Session::open(&directory)?;
    let stream = transport::Connection::connect(std::path::Path::new(&socket))?;
    while let Some((kind, body)) = stream.receive()? {
        if kind == -1 || (kind == -3 && body == b"0") {
            break;
        }
        let request = match kind {
            -2 => Ok(Request::Refresh {}),
            1 => serde_json::from_slice::<Request>(&body),
            _ => continue,
        };
        let state = match request
            .map_err(anyhow::Error::from)
            .and_then(|r| session.handle(r))
        {
            Ok(state) => state,
            Err(error) => State::Message {
                title: "Anki".into(),
                body: error.to_string(),
            },
        };
        stream.send(101, &serde_json::to_vec(&state)?)?;
    }
    Ok(())
}
