use std::io::Write;
use std::hash::{Hash, Hasher};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::backend::{Backend, DeckInfo, Grade, ReviewCard, SyncAccount};
use crate::render::Renderer;

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Home {},
    Refresh {},
    Review {
        deck_id: String,
    },
    Preview {
        deck_id: String,
    },
    Previous {},
    Next {},
    Reveal {},
    Answer {
        grade: Rating,
    },
    Undo {},
    Flag {
        flag: u8,
    },
    Bury {},
    Suspend {},
    Sync {},
    RecordNotebook {
        book_id: String,
    },
    RecordSheet {
        card_id: String,
        book_id: String,
        page_id: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

impl Rating {
    fn grade(self) -> Grade {
        match self {
            Self::Again => Grade::Again,
            Self::Hard => Grade::Hard,
            Self::Good => Grade::Good,
            Self::Easy => Grade::Easy,
        }
    }
}

#[derive(Serialize)]
pub struct Deck {
    id: String,
    name: String,
    level: u32,
    has_children: bool,
    new: u32,
    learn: u32,
    due: u32,
    total: usize,
}

impl From<DeckInfo> for Deck {
    fn from(d: DeckInfo) -> Self {
        Self {
            id: d.id.to_string(),
            name: d.name,
            level: d.level,
            has_children: d.has_children,
            new: d.new,
            learn: d.learn,
            due: d.review,
            total: 0,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum State {
    Home {
        decks: Vec<Deck>,
    },
    Card {
        card_id: String,
        deck_id: String,
        deck_name: String,
        image: String,
        question_image: String,
        answer_image: String,
        image_height: u32,
        revealed: bool,
        intervals: [String; 4],
        flag: u8,
        new: usize,
        learn: usize,
        due: usize,
        book_id: Option<String>,
        page_id: Option<String>,
        preview: Option<PreviewPosition>,
    },
    Finished {
        deck_name: String,
        deck_id: String,
        total: usize,
    },
    Message {
        title: String,
        body: String,
    },
}

#[derive(Serialize, Deserialize)]
struct ImageCache {
    signature: u64,
    heights: [u32; 2],
}

pub struct Session {
    backend: Backend,
    renderer: Renderer,
    directory: PathBuf,
    deck_id: Option<String>,
    deck_name: String,
    current: Option<ActiveCard>,
    preview_ids: Vec<i64>,
    preview_index: usize,
    revealed: bool,
    heights: [u32; 2],
    media_revision: u64,
    notes: NativeNotes,
}

#[derive(Serialize)]
pub struct PreviewPosition {
    number: usize,
    total: usize,
}

enum ActiveCard {
    Review(ReviewCard),
    Preview {
        id: i64,
        question: String,
        answer: String,
    },
}

impl ActiveCard {
    fn id(&self) -> i64 {
        match self {
            Self::Review(card) => card.card_id.0,
            Self::Preview { id, .. } => *id,
        }
    }
    fn html(&self) -> (&str, &str) {
        match self {
            Self::Review(card) => (&card.question_html, &card.answer_html),
            Self::Preview {
                question, answer, ..
            } => (question, answer),
        }
    }
    fn review(&self) -> Result<&ReviewCard> {
        match self {
            Self::Review(card) => Ok(card),
            Self::Preview { .. } => bail!("Preview does not change scheduling"),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeNotes {
    book_id: Option<String>,
    page_id: Option<String>,
}

fn uuid_valid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

fn media_revision(directory: &Path) -> u64 {
    let mut files: Vec<_> = std::fs::read_dir(directory)
        .into_iter().flatten().flatten().map(|entry| entry.path()).collect();
    files.sort();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for path in files {
        path.file_name().hash(&mut hasher);
        if let Ok(metadata) = path.metadata() {
            metadata.len().hash(&mut hasher);
            metadata.modified().ok().hash(&mut hasher);
        }
    }
    hasher.finish()
}

impl Session {
    pub fn open(directory: &Path) -> Result<Self> {
        std::fs::create_dir_all(directory)?;
        let path = directory.join("collection.anki2");
        if !path.exists() {
            crate::demo::create(&path)?;
        }
        let backend = Backend::open(path.to_str().context("Invalid collection path")?)?;
        let notes_path = directory.join("native-notes.json");
        let notes = if notes_path.exists() {
            serde_json::from_slice(&std::fs::read(notes_path)?)?
        } else {
            NativeNotes::default()
        };
        Ok(Self {
            backend,
            renderer: Renderer::new().with_media_dir(path.with_extension("media")),
            directory: directory.to_owned(),
            deck_id: None,
            deck_name: String::new(),
            current: None,
            preview_ids: Vec::new(),
            preview_index: 0,
            revealed: false,
            heights: [0; 2],
            media_revision: media_revision(&path.with_extension("media")),
            notes,
        })
    }

    pub fn prepare_cards(&mut self) -> Result<usize> {
        let decks = self.backend.deck_infos()?;
        let mut ids = std::collections::BTreeSet::new();
        for deck in decks.into_iter().filter(|deck| deck.level == 0) {
            ids.extend(self.backend.card_ids_in_deck(deck.id)?);
        }
        let total = ids.len();
        for (index, id) in ids.into_iter().enumerate() {
            let (question, answer) = self.backend.study_card_html(id)?;
            self.current = Some(ActiveCard::Preview { id, question, answer });
            self.render_current()?;
            println!("Prepared {} / {}", index + 1, total);
        }
        self.current = None;
        Ok(total)
    }

    pub fn handle(&mut self, request: Request) -> Result<State> {
        match request {
            Request::Home {} => {
                self.deck_id = None;
                self.current = None;
                self.preview_ids.clear();
            }
            Request::Refresh {} => {}
            Request::Review { deck_id } => {
                let id = deck_id.parse().context("Invalid deck ID")?;
                let deck = self
                    .backend
                    .deck_infos()?
                    .into_iter()
                    .find(|d| d.id == id)
                    .context("Deck no longer exists")?;
                self.backend.set_deck_scope_id(id)?;
                self.preview_ids.clear();
                self.deck_id = Some(deck_id);
                self.deck_name = deck.name;
                self.advance()?;
            }
            Request::Preview { deck_id } => {
                let id = deck_id.parse().context("Invalid deck ID")?;
                let deck = self
                    .backend
                    .deck_infos()?
                    .into_iter()
                    .find(|deck| deck.id == id)
                    .context("Deck no longer exists")?;
                self.backend.set_deck_scope_id(id)?;
                self.preview_ids = self.backend.card_ids_in_deck(id)?;
                self.preview_index = 0;
                self.deck_id = Some(deck_id);
                self.deck_name = deck.name;
                self.open_preview()?;
            }
            Request::Previous {} => {
                if !matches!(self.current, Some(ActiveCard::Preview { .. })) {
                    bail!("Open preview first");
                }
                self.preview_index = self.preview_index.saturating_sub(1);
                self.open_preview()?;
            }
            Request::Next {} => {
                if !matches!(self.current, Some(ActiveCard::Preview { .. })) {
                    bail!("Open preview first");
                }
                self.preview_index = (self.preview_index + 1).min(self.preview_ids.len() - 1);
                self.open_preview()?;
            }
            Request::Reveal {} => {
                if self.current.is_none() {
                    bail!("Choose a deck first");
                }
                self.revealed = true;
            }
            Request::Answer { grade } => {
                if !self.revealed {
                    bail!("Reveal the answer before rating it");
                }
                self.backend.answer(
                    self.current.as_ref().context("No current card")?.review()?,
                    grade.grade(),
                )?;
                self.advance()?;
            }
            Request::Undo {} => {
                if self.backend.undo()? && self.deck_id.is_some() {
                    self.advance()?;
                }
            }
            Request::Flag { flag } => {
                if flag > 7 {
                    bail!("Flags range from 0 to 7");
                }
                let Some(ActiveCard::Review(card)) = self.current.as_mut() else {
                    bail!("Open review first");
                };
                self.backend.set_flag(card.card_id, flag as u32)?;
                card.flags = flag;
            }
            Request::Bury {} => {
                self.backend.bury_card(
                    self.current
                        .as_ref()
                        .context("No current card")?
                        .review()?
                        .card_id,
                )?;
                self.advance()?;
            }
            Request::Suspend {} => {
                self.backend.suspend_note(
                    self.current
                        .as_ref()
                        .context("No current card")?
                        .review()?
                        .note_id,
                )?;
                self.advance()?;
            }
            Request::Sync {} => {
                let account: SyncAccount = serde_json::from_slice(&std::fs::read(self.directory.join("sync-auth.json"))
                    .context("Connect your desktop Anki with ./ankipure connect-anki, or run ./ankipure sync-setup")?)?;
                let report = self.backend.sync_account(&account)?;
                self.deck_id = None;
                self.current = None;
                return Ok(State::Message {
                    title: report.headline,
                    body: [
                        report.added,
                        report.removed,
                        report.media,
                        report.server_message,
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join("\n"),
                });
            }
            Request::RecordNotebook { book_id } => {
                self.record_notebook(book_id)?;
                self.save_notes()?;
            }
            Request::RecordSheet {
                card_id,
                book_id,
                page_id,
            } => {
                let current = self.current.as_ref().context("No current card")?;
                if current.id().to_string() != card_id
                    || !uuid_valid(&book_id)
                    || !uuid_valid(&page_id)
                {
                    bail!("Invalid answer page association");
                }
                self.record_notebook(book_id)?;
                self.notes.page_id = Some(page_id);
                self.save_notes()?;
            }
        }
        self.state()
    }

    fn record_notebook(&mut self, book_id: String) -> Result<()> {
        if !uuid_valid(&book_id) {
            bail!("Invalid notebook ID");
        }
        if self.notes.book_id.as_ref().is_some_and(|id| id != &book_id) {
            bail!("All writing belongs in the existing Anki notebook");
        }
        self.notes.book_id = Some(book_id);
        Ok(())
    }

    fn save_notes(&self) -> Result<()> {
        let path = self.directory.join("native-notes.json");
        let temporary = path.with_extension("tmp");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec(&self.notes)?)?;
        file.sync_all()?;
        std::fs::rename(temporary, path)?;
        Ok(())
    }

    fn advance(&mut self) -> Result<()> {
        let mut card = self.backend.next_card()?;
        if let Some(card) = &mut card {
            card.answer_html = self.backend.study_card_html(card.card_id.0)?.1;
        }
        self.current = card.map(ActiveCard::Review);
        self.render_current()
    }

    fn open_preview(&mut self) -> Result<()> {
        self.current = if let Some(&id) = self.preview_ids.get(self.preview_index) {
            let (question, answer) = self.backend.study_card_html(id)?;
            Some(ActiveCard::Preview {
                id,
                question,
                answer,
            })
        } else {
            None
        };
        self.render_current()
    }

    fn render_current(&mut self) -> Result<()> {
        self.revealed = false;
        if let Some(card) = &self.current {
            let (question, answer) = card.html();
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            ("paper-36-v1", question, answer, self.media_revision).hash(&mut hasher);
            let signature = hasher.finish();
            let cache_path = self.directory.join(format!("card-{}.json", card.id()));
            let cached = std::fs::read(&cache_path).ok()
                .and_then(|bytes| serde_json::from_slice::<ImageCache>(&bytes).ok());
            if let Some(cache) = cached.filter(|cache| cache.signature == signature
                && (0..2).all(|side| self.directory.join(format!("card-{}-{side}.png", card.id())).is_file())) {
                self.heights = cache.heights;
                return Ok(());
            }
            for (index, html) in [question, answer].into_iter().enumerate() {
                let width = crate::device::PANEL_WIDTH as u32;
                let view = self.renderer.build_card_view_at(html, width, 400, 0, false);
                let height = (view.content_h().ceil() as u32).max(400);
                let mut pixels = view.paint_height(height);
                for p in pixels.chunks_exact_mut(4) {
                    let gray = ((p[0] as u32 * 2126 + p[1] as u32 * 7152 + p[2] as u32 * 722)
                        / 10000) as u8;
                    p[..3].fill(gray);
                }
                let path = self
                    .directory
                    .join(format!("card-{}-{index}.png", card.id()));
                let temporary = path.with_extension("tmp");
                let mut encoder = png::Encoder::new(std::fs::File::create(&temporary)?, width, height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.set_compression(png::Compression::Fast);
                encoder.write_header()?.write_image_data(&pixels)?;
                std::fs::rename(temporary, path)?;
                self.heights[index] = height;
            }
            let temporary = cache_path.with_extension("tmp");
            std::fs::write(&temporary, serde_json::to_vec(&ImageCache { signature, heights: self.heights })?)?;
            std::fs::rename(temporary, cache_path)?;
        }
        Ok(())
    }

    fn state(&mut self) -> Result<State> {
        let Some(deck_id) = &self.deck_id else {
            let mut decks: Vec<Deck> = self
                .backend
                .deck_infos()?
                .into_iter()
                .map(Deck::from)
                .collect();
            for deck in &mut decks {
                deck.total = self.backend.card_ids_in_deck(deck.id.parse()?)?.len();
            }
            return Ok(State::Home { decks });
        };
        let Some(card) = &self.current else {
            return Ok(State::Finished {
                deck_name: self.deck_name.clone(),
                deck_id: deck_id.clone(),
                total: self.backend.card_ids_in_deck(deck_id.parse()?)?.len(),
            });
        };
        let counts = self.backend.counts()?;
        let side = usize::from(self.revealed);
        let image_url = |side| {
            format!(
                "file://{}",
                self.directory
                    .join(format!("card-{}-{side}.png", card.id()))
                    .display()
            )
        };
        Ok(State::Card {
            card_id: card.id().to_string(),
            deck_id: deck_id.clone(),
            deck_name: self.deck_name.clone(),
            image: image_url(side),
            question_image: image_url(0),
            answer_image: image_url(1),
            image_height: self.heights[side],
            revealed: self.revealed,
            intervals: match card {
                ActiveCard::Review(card) => card.button_labels.clone(),
                _ => Default::default(),
            },
            flag: match card {
                ActiveCard::Review(card) => card.flags,
                _ => 0,
            },
            new: counts.new,
            learn: counts.learning,
            due: counts.review,
            book_id: self.notes.book_id.clone(),
            page_id: self.notes.page_id.clone(),
            preview: if matches!(card, ActiveCard::Preview { .. }) {
                Some(PreviewPosition {
                    number: self.preview_index + 1,
                    total: self.preview_ids.len(),
                })
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_cache_survives_reopening_and_invalidates_when_content_changes() {
        let dir = std::env::temp_dir().join(format!("pure-render-cache-{}", std::process::id()));
        let mut session = Session::open(&dir).unwrap();
        session.current = Some(ActiveCard::Preview { id: 99, question: "Question".into(), answer: "Answer".into() });
        session.render_current().unwrap();
        let path = dir.join("card-99-1.png");
        let before = std::fs::metadata(&path).unwrap().modified().unwrap();
        drop(session);
        let mut session = Session::open(&dir).unwrap();
        session.current = Some(ActiveCard::Preview { id: 99, question: "Question".into(), answer: "Answer".into() });
        session.render_current().unwrap();
        assert_eq!(before, std::fs::metadata(&path).unwrap().modified().unwrap());
        let previous = std::fs::read(&path).unwrap();
        session.current = Some(ActiveCard::Preview { id: 99, question: "Question".into(), answer: "Changed answer".into() });
        session.render_current().unwrap();
        assert_ne!(previous, std::fs::read(&path).unwrap());
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn long_cards_are_not_cut_off_at_twelve_thousand_pixels() {
        let dir = std::env::temp_dir().join(format!("pure-long-card-{}", std::process::id()));
        let mut session = Session::open(&dir).unwrap();
        session.current = Some(ActiveCard::Preview { id: 99, question: "Question".into(),
            answer: "<p>Worked example line</p>".repeat(190) });
        session.render_current().unwrap();
        assert!(session.heights[1] > 12000, "{}", session.heights[1]);
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn notebook_identity_is_saved_before_the_first_page_and_cannot_be_replaced() {
        let dir = std::env::temp_dir().join(format!("pure-book-identity-{}", std::process::id()));
        let mut session = Session::open(&dir).unwrap();
        let book_id = "11111111-1111-4111-8111-111111111111".to_string();
        assert!(session
            .handle(Request::RecordNotebook {
                book_id: "invalid".into()
            })
            .is_err());
        session
            .handle(Request::RecordNotebook {
                book_id: book_id.clone(),
            })
            .unwrap();
        drop(session);
        let mut session = Session::open(&dir).unwrap();
        assert_eq!(session.notes.book_id, Some(book_id));
        assert_eq!(session.notes.page_id, None);
        assert!(session
            .handle(Request::RecordNotebook {
                book_id: "22222222-2222-4222-8222-222222222222".into()
            })
            .is_err());
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn every_deck_and_card_reuses_one_scratch_page() {
        let dir = std::env::temp_dir().join(format!("pure-one-notebook-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("collection.anki2");
        crate::demo::create(&path).unwrap();
        let mut collection = crate::backend::open_collection(&path).unwrap();
        let other = collection
            .get_or_create_normal_deck("Another deck")
            .unwrap();
        let basic = collection.get_notetype_by_name("Basic").unwrap().unwrap();
        let mut note = basic.new_note();
        note.set_field(0, "A different deck").unwrap();
        note.set_field(1, "The same notebook").unwrap();
        collection.add_note(&mut note, other.id).unwrap();
        drop(collection);
        let mut session = Session::open(&dir).unwrap();
        let State::Home { decks } = session.handle(Request::Home {}).unwrap() else {
            panic!("home")
        };
        let first = decks
            .iter()
            .find(|deck| deck.total == 5)
            .unwrap()
            .id
            .clone();
        let State::Card { card_id, .. } = session
            .handle(Request::Preview {
                deck_id: first.clone(),
            })
            .unwrap()
        else {
            panic!("card")
        };
        let book = "11111111-1111-4111-8111-111111111111".to_string();
        let page = "22222222-2222-4222-8222-222222222222".to_string();
        session
            .handle(Request::RecordSheet {
                card_id,
                book_id: book.clone(),
                page_id: page.clone(),
            })
            .unwrap();
        let State::Card {
            book_id, page_id, ..
        } = session
            .handle(Request::Preview {
                deck_id: other.id.0.to_string(),
            })
            .unwrap()
        else {
            panic!("other deck")
        };
        assert_eq!(book_id, Some(book.clone()));
        assert_eq!(page_id, Some(page.clone()));
        drop(session);
        let mut session = Session::open(&dir).unwrap();
        let State::Card {
            book_id, page_id, ..
        } = session.handle(Request::Preview { deck_id: first }).unwrap()
        else {
            panic!("reopened card")
        };
        assert_eq!(book_id, Some(book));
        assert_eq!(page_id, Some(page));
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reveal_and_sheet_association_reuse_both_prepared_card_images() {
        let dir = std::env::temp_dir().join(format!("pure-reveal-{}", std::process::id()));
        let mut session = Session::open(&dir).unwrap();
        let State::Home { decks } = session.handle(Request::Home {}).unwrap() else {
            panic!("home");
        };
        let deck = decks.into_iter().find(|deck| deck.total > 0).unwrap();
        let State::Card {
            card_id,
            question_image,
            answer_image,
            image,
            ..
        } = session
            .handle(Request::Review { deck_id: deck.id })
            .unwrap()
        else {
            panic!("card");
        };
        assert_eq!(image, question_image);
        let paths = [&question_image, &answer_image]
            .map(|url| PathBuf::from(url.strip_prefix("file://").unwrap()));
        let timestamps = paths
            .each_ref()
            .map(|path| std::fs::metadata(path).unwrap().modified().unwrap());
        let State::Card {
            revealed, image, ..
        } = session.handle(Request::Reveal {}).unwrap()
        else {
            panic!("revealed card");
        };
        assert!(revealed);
        assert_eq!(image, answer_image);
        session
            .handle(Request::RecordSheet {
                card_id,
                book_id: "11111111-1111-1111-1111-111111111111".into(),
                page_id: "22222222-2222-2222-2222-222222222222".into(),
            })
            .unwrap();
        assert_eq!(
            timestamps,
            paths
                .each_ref()
                .map(|path| std::fs::metadata(path).unwrap().modified().unwrap())
        );
        drop(session);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn preview_displays_future_cards_without_grading_or_changing_scheduling() {
        let dir = std::env::temp_dir().join(format!("pure-preview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        crate::demo::create(&dir.join("collection.anki2")).unwrap();
        let db = rusqlite::Connection::open(dir.join("collection.anki2")).unwrap();
        db.execute("update cards set queue=2,type=2,due=100000", [])
            .unwrap();
        drop(db);
        let mut session = Session::open(&dir).unwrap();
        let State::Home { decks } = session.handle(Request::Home {}).unwrap() else {
            panic!("home");
        };
        let deck = decks.into_iter().find(|deck| deck.total == 5).unwrap();
        assert_eq!(deck.new + deck.learn + deck.due, 0);
        assert!(matches!(
            session
                .handle(Request::Review {
                    deck_id: deck.id.clone()
                })
                .unwrap(),
            State::Finished { total: 5, .. }
        ));
        let State::Card {
            card_id,
            image,
            preview: Some(position),
            ..
        } = session
            .handle(Request::Preview { deck_id: deck.id })
            .unwrap()
        else {
            panic!("preview");
        };
        assert_eq!((position.number, position.total), (1, 5));
        let decoder =
            png::Decoder::new(std::fs::File::open(image.strip_prefix("file://").unwrap()).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        let frame = reader.next_frame(&mut pixels).unwrap();
        assert!(
            pixels[..frame.buffer_size()]
                .chunks_exact(4)
                .filter(|pixel| pixel[0] < 100 && pixel[3] > 0)
                .count()
                > 100
        );
        session.handle(Request::Reveal {}).unwrap();
        assert!(session
            .handle(Request::Answer {
                grade: Rating::Good
            })
            .is_err());
        let State::Card {
            card_id: next,
            revealed,
            ..
        } = session.handle(Request::Next {}).unwrap()
        else {
            panic!("next");
        };
        assert_ne!(next, card_id);
        assert!(!revealed);
        let State::Card {
            card_id: previous, ..
        } = session.handle(Request::Previous {}).unwrap()
        else {
            panic!("previous");
        };
        assert_eq!(previous, card_id);
        drop(session);
        let db = rusqlite::Connection::open(dir.join("collection.anki2")).unwrap();
        assert_eq!(
            db.query_row(
                "select count(*) from cards where queue=2 and due=100000",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            5
        );
        assert_eq!(
            db.query_row("select count(*) from revlog", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_session_requires_reveal_keeps_ids_exact_and_saves_ratings() {
        let dir = std::env::temp_dir().join(format!("pure-native-session-{}", std::process::id()));
        let mut s = Session::open(&dir).unwrap();
        let State::Home { decks } = s.handle(Request::Home {}).unwrap() else {
            panic!("home expected")
        };
        let deck = decks
            .into_iter()
            .find(|d| d.new + d.learn + d.due > 0)
            .unwrap();
        let State::Card { card_id, image, .. } =
            s.handle(Request::Review { deck_id: deck.id }).unwrap()
        else {
            panic!("card expected")
        };
        assert!(card_id.parse::<i64>().unwrap() > 0);
        assert!(Path::new(image.strip_prefix("file://").unwrap()).exists());
        assert!(s
            .handle(Request::Answer {
                grade: Rating::Good
            })
            .is_err());
        let State::Card {
            card_id: revealed,
            revealed: is_answer,
            ..
        } = s.handle(Request::Reveal {}).unwrap()
        else {
            panic!("answer expected")
        };
        assert_eq!(revealed, card_id);
        assert!(is_answer);
        s.handle(Request::Answer {
            grade: Rating::Good,
        })
        .unwrap();
        s.handle(Request::Undo {}).unwrap();
        drop(s);
        let db = rusqlite::Connection::open(dir.join("collection.anki2")).unwrap();
        assert_eq!(
            db.query_row("select count(*) from revlog", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(db);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn protocol_rejects_unknown_actions_and_invalid_ratings() {
        assert!(
            serde_json::from_str::<Request>(r#"{"action":"answer","grade":"excellent"}"#).is_err()
        );
        assert!(serde_json::from_str::<Request>(
            r#"{"action":"review","deck_id":"9007199254740993"}"#
        )
        .is_ok());
        assert!(serde_json::from_str::<Request>(r#"{"action":"home","extra":1}"#).is_err());
    }

    #[test]
    fn native_page_associations_are_private_atomic_and_survive_reopening() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("pure-native-pages-{}", std::process::id()));
        let mut s = Session::open(&dir).unwrap();
        let State::Home { decks } = s.handle(Request::Home {}).unwrap() else {
            panic!("home expected")
        };
        let deck = decks.into_iter().find(|d| d.new > 0).unwrap();
        let State::Card { card_id, .. } = s.handle(Request::Review { deck_id: deck.id }).unwrap()
        else {
            panic!("card expected")
        };
        let book_id = "11111111-1111-4111-8111-111111111111".to_string();
        let page_id = "22222222-2222-4222-8222-222222222222".to_string();
        assert!(s
            .handle(Request::RecordSheet {
                card_id: card_id.clone(),
                book_id: "../notes".into(),
                page_id: page_id.clone()
            })
            .is_err());
        s.handle(Request::RecordSheet {
            card_id: card_id.clone(),
            book_id: book_id.clone(),
            page_id: page_id.clone(),
        })
        .unwrap();
        let path = dir.join("native-notes.json");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(!dir.join("native-notes.tmp").exists());
        drop(s);
        let reopened = Session::open(&dir).unwrap();
        assert_eq!(reopened.notes.book_id, Some(book_id));
        assert_eq!(reopened.notes.page_id, Some(page_id));
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
