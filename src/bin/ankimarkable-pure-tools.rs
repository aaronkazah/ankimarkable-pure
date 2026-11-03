use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use anki_proto::import_export::{ExportAnkiPackageOptions, ImportAnkiPackageOptions};
use ankimarkable_pure::backend::open_collection;
use ankimarkable_pure::backend::Backend;
use ankimarkable_pure::session::{Request, Session, State};
use ankimarkable_pure::{demo, device, render::Renderer};
use anyhow::{bail, Context, Result};

fn save_png(path: &Path, pixels: &[u8]) -> Result<()> {
    let mut encoder = png::Encoder::new(
        std::fs::File::create(path)?,
        device::PANEL_WIDTH as u32,
        400,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(pixels)?;
    Ok(())
}

fn preview(collection: &Path, directory: &Path) -> Result<()> {
    std::fs::create_dir_all(directory)?;
    let renderer = Renderer::new().with_media_dir(collection.with_extension("media"));
    let mut backend = Backend::open(collection.to_str().context("Invalid collection path")?)?;
    let deck = backend
        .deck_infos()?
        .into_iter()
        .find(|d| d.new + d.learn + d.review > 0)
        .context("No due cards")?;
    backend.set_deck_scope_id(deck.id)?;
    let card = backend.next_card()?.context("No due cards")?;
    for (name, html) in [
        ("question.png", &card.question_html),
        ("answer.png", &card.answer_html),
    ] {
        let pixels = renderer
            .build_card_view_at(html, device::PANEL_WIDTH as u32, 400, 0, false)
            .paint_window();
        save_png(&directory.join(name), &pixels)?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["configure-sync", directory] => {
            let input = std::io::read_to_string(std::io::stdin())?;
            let mut lines = input.lines();
            let username = lines.next().context("Missing email")?;
            let password = lines.next().context("Missing password")?;
            let endpoint = lines
                .next()
                .filter(|line| !line.is_empty())
                .map(str::to_owned);
            let account = Backend::login(username, password, endpoint)?;
            let path = Path::new(directory).join("sync-auth.json");
            let temporary = path.with_extension("tmp");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?;
            file.write_all(&serde_json::to_vec(&account)?)?;
            file.sync_all()?;
            std::fs::rename(temporary, path)?;
            println!("Sync connected. The password was not saved.");
        }
        ["sync", directory] => {
            let mut session = Session::open(Path::new(directory))?;
            let State::Message { title, body } = session.handle(Request::Sync {})? else {
                bail!("Unexpected sync result");
            };
            println!("{title}\n{body}");
        }
        ["migrate-config", manifest, directory] => {
            let data: serde_json::Value = serde_json::from_slice(&std::fs::read(manifest)?)?;
            if let Some(endpoint) = data
                .pointer("/environment/ANKIMARKABLE_SYNC_ENDPOINT")
                .and_then(|v| v.as_str())
            {
                let path = Path::new(directory).join("sync-endpoint");
                if !path.exists() {
                    let mut file = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .open(path)?;
                    file.write_all(endpoint.as_bytes())?;
                }
            }
        }
        ["export", collection, package] => {
            let mut collection = open_collection(Path::new(collection))?;
            collection.export_apkg(
                package,
                ExportAnkiPackageOptions {
                    with_scheduling: true,
                    with_deck_configs: true,
                    with_media: true,
                    legacy: false,
                },
                "",
                None,
            )?;
            println!("Deck exported with scheduling and media.");
        }
        ["import", collection, package] => {
            let mut collection = open_collection(Path::new(collection))?;
            collection.import_apkg(
                package,
                ImportAnkiPackageOptions {
                    with_scheduling: true,
                    with_deck_configs: true,
                    ..Default::default()
                },
            )?;
            println!("Deck imported, including media and exported scheduling.");
        }
        ["create-demo", collection] => {
            demo::create(Path::new(collection))?;
            println!("Created a separate sample collection.");
        }
        ["preview", collection, directory] => preview(Path::new(collection), Path::new(directory))?,
        ["prepare", directory] => {
            let count = Session::open(Path::new(directory))?.prepare_cards()?;
            println!("Prepared {count} cards without changing their reviews.");
        }
        ["check", collection] => {
            let mut backend = Backend::open(collection)?;
            for deck in backend.deck_infos()? {
                println!(
                    "{}: {} new, {} learning, {} review",
                    deck.name, deck.new, deck.learn, deck.review
                );
            }
        }
        _ => bail!("Use ./ankipure help for setup, import, export, build, and test commands."),
    }
    Ok(())
}
