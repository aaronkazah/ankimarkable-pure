use std::path::Path;

use crate::backend::open_collection;
use anyhow::{bail, Context, Result};

pub const DECK_NAME: &str = "Paper Pure Demo";

pub fn create(path: &Path) -> Result<()> {
    if path.exists() {
        bail!("Collection already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut collection = open_collection(path)?;
    let deck = collection.get_or_create_normal_deck(DECK_NAME)?;
    let basic = collection
        .get_notetype_by_name("Basic")?
        .context("Basic note type missing")?;
    for (question, answer) in [
        (
            "What is 7 × 8?<br><br>Write your answer with the pen, then tap Show Answer.",
            "56",
        ),
        (
            "Translate <b>bonjour</b> into English.",
            "Hello / good morning",
        ),
        (
            "How many small squares are in this grid?<br><img src=\"demo-grid.png\">",
            "4 small squares; 5 squares including the outside square.",
        ),
        (
            r"Simplify \(\frac{1}{2} + \frac{1}{4}\).",
            r"\(\frac{3}{4}\)",
        ),
    ] {
        let mut note = basic.new_note();
        note.set_field(0, question)?;
        note.set_field(1, answer)?;
        collection.add_note(&mut note, deck.id)?;
    }
    let cloze = collection
        .get_notetype_by_name("Cloze")?
        .context("Cloze note type missing")?;
    let mut note = cloze.new_note();
    note.set_field(0, "Water freezes at {{c1::0 °C}} at standard pressure.")?;
    note.set_field(1, "A cloze card hides part of a sentence.")?;
    collection.add_note(&mut note, deck.id)?;
    drop(collection);

    let media_dir = path.with_extension("media");
    std::fs::create_dir_all(&media_dir)?;
    let file = std::fs::File::create(media_dir.join("demo-grid.png"))?;
    let mut encoder = png::Encoder::new(file, 256, 256);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut pixels = vec![255; 256 * 256];
    for y in 30..226 {
        for x in 30..226 {
            if [30, 126, 222]
                .iter()
                .any(|line| (x >= *line && x < *line + 4) || (y >= *line && y < *line + 4))
            {
                pixels[y * 256 + x] = 0;
            }
        }
    }
    encoder.write_header()?.write_image_data(&pixels)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Backend, Grade};

    #[test]
    fn demo_reviews_are_saved_and_undo_restores_the_card() {
        let directory =
            std::env::temp_dir().join(format!("pure-review-contract-{}", std::process::id()));
        let path = directory.join("collection.anki2");
        create(&path).unwrap();
        assert!(create(&path).is_err());
        let mut backend = Backend::open(path.to_str().unwrap()).unwrap();
        backend.set_deck_scope(DECK_NAME).unwrap();
        let card = backend.next_card().unwrap().unwrap();
        backend.answer(&card, Grade::Good).unwrap();
        assert!(backend.undo().unwrap());
        assert_eq!(backend.next_card().unwrap().unwrap().card_id, card.card_id);
        backend.answer(&card, Grade::Good).unwrap();
        drop(backend);
        let connection = rusqlite::Connection::open(&path).unwrap();
        let reviews: i64 = connection
            .query_row("SELECT count(*) FROM revlog", [], |row| row.get(0))
            .unwrap();
        assert_eq!(reviews, 1);
        drop(connection);
        let mut reopened = Backend::open(path.to_str().unwrap()).unwrap();
        reopened.set_deck_scope(DECK_NAME).unwrap();
        assert_ne!(reopened.next_card().unwrap().unwrap().card_id, card.card_id);
        drop(reopened);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
