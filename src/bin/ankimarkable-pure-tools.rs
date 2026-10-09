use std::path::Path;

use anki_proto::import_export::{ExportAnkiPackageOptions, ImportAnkiPackageOptions};
use ankimarkable_pure::backend::open_collection;
use ankimarkable_pure::backend::{Backend, Stats};
use ankimarkable_pure::{demo, render::Renderer, ui};
use anyhow::{bail, Context, Result};

fn save_png(path: &Path, pixels: &[u8]) -> Result<()> {
    let mut encoder = png::Encoder::new(
        std::fs::File::create(path)?,
        ui::WIDTH as u32,
        ui::HEIGHT as u32,
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
    let decks = backend.deck_infos()?;
    save_png(
        &directory.join("home.png"),
        &ui::compose_home(&renderer, &decks, 0, Stats::default()),
    )?;
    let deck = decks
        .iter()
        .find(|deck| deck.new + deck.learn + deck.review > 0)
        .context("No due cards")?;
    backend.set_deck_scope_id(deck.id)?;
    let counts = backend.counts()?;
    let card = backend.next_card()?.context("No due cards")?;
    for (name, html, phase) in [
        ("question.png", &card.question_html, ui::Phase::Question),
        ("answer.png", &card.answer_html, ui::Phase::Answer),
    ] {
        let window = renderer
            .build_card_view_at(html, ui::WIDTH as u32, ui::CARD_H as u32, 0, false)
            .paint_window();
        save_png(
            &directory.join(name),
            &ui::compose_review(
                &renderer,
                &window,
                &card,
                phase,
                counts,
                &deck.name,
                false,
                &ankimarkable_pure::prefs::Prefs::default(),
                ui::MenuState::Closed,
            ),
        )?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["export", collection, package] => {
            let mut collection = open_collection(Path::new(collection))?;
            collection.export_apkg(package, ExportAnkiPackageOptions { with_scheduling: true, with_deck_configs: true, with_media: true, legacy: false }, "", None)?;
            println!("Deck exported with scheduling and media.");
        }
        ["import", collection, package] => {
            let mut collection = open_collection(Path::new(collection))?;
            collection.import_apkg(package, ImportAnkiPackageOptions { with_scheduling: true, with_deck_configs: true, ..Default::default() })?;
            println!("Deck imported, including media and exported scheduling.");
        }
        ["create-demo", collection] => { demo::create(Path::new(collection))?; println!("Created a separate sample collection."); }
        ["preview", collection, directory] => preview(Path::new(collection), Path::new(directory))?,
        ["check", collection] => {
            let mut backend = Backend::open(collection)?;
            for deck in backend.deck_infos()? { println!("{}: {} new, {} learning, {} review", deck.name, deck.new, deck.learn, deck.review); }
        }
        _ => bail!("Usage: ankimarkable-pure-tools create-demo COLLECTION | export COLLECTION PACKAGE.apkg | import COLLECTION PACKAGE.apkg | preview COLLECTION DIRECTORY | check COLLECTION"),
    }
    Ok(())
}
