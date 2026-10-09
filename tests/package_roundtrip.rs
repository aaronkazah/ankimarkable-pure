use anki::search::SortMode;
use anki_proto::import_export::{ExportAnkiPackageOptions, ImportAnkiPackageOptions};
use ankimarkable_pure::backend::open_collection;
use ankimarkable_pure::demo;

#[test]
fn current_anki_packages_preserve_notes_and_image_media() {
    let directory = std::env::temp_dir().join(format!("pure-package-{}", std::process::id()));
    let source = directory.join("source.anki2");
    let target = directory.join("target.anki2");
    let package = directory.join("demo.apkg");
    demo::create(&source).unwrap();
    let mut collection = open_collection(&source).unwrap();
    collection
        .export_apkg(
            &package,
            ExportAnkiPackageOptions {
                with_scheduling: true,
                with_deck_configs: true,
                with_media: true,
                legacy: false,
            },
            "",
            None,
        )
        .unwrap();
    drop(collection);
    let mut imported = open_collection(&target).unwrap();
    imported
        .import_apkg(
            &package,
            ImportAnkiPackageOptions {
                with_scheduling: true,
                with_deck_configs: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        imported.search_cards("", SortMode::NoOrder).unwrap().len(),
        5
    );
    assert_eq!(
        std::fs::read(source.with_extension("media").join("demo-grid.png")).unwrap(),
        std::fs::read(target.with_extension("media").join("demo-grid.png")).unwrap()
    );
    drop(imported);
    std::fs::remove_dir_all(directory).unwrap();
}
