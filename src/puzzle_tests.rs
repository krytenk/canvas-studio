use super::*;
use crate::puzzle_production as puzzles;
use std::{fs, process::Command};

#[test]
fn puzzles_are_deterministic_and_exhaustively_checked_across_seeds_and_difficulties() {
    for difficulty in [
        puzzles::Difficulty::Easy,
        puzzles::Difficulty::Medium,
        puzzles::Difficulty::Hard,
    ] {
        for seed in 0..60 {
            let config = puzzles::WordSearchConfig {
                seed,
                difficulty,
                ..Default::default()
            };
            let (grid, placements) = puzzles::generate(&config).unwrap();
            assert_eq!(
                (grid.clone(), placements.clone()),
                puzzles::generate(&config).unwrap()
            );
            puzzles::validate(&config, &grid, &placements).unwrap();
            // A deliberately changed solution must fail verification.
            let mut invalid = placements.clone();
            invalid[0].word = "NOTAWORDFIXTURE".into();
            assert!(puzzles::validate(&config, &grid, &invalid).is_err());
            for word in &config.words {
                assert_eq!(
                    puzzles::occurrences(&grid, &puzzles::normalise(word).unwrap()).len(),
                    1
                );
            }
        }
    }
}
#[test]
fn puzzle_validation_rejects_duplicates_accidental_matches_and_bad_input_without_mutation() {
    let mut config = puzzles::WordSearchConfig::default();
    for words in [
        vec!["ROSE", "ROSE"],
        vec!["STOP", "POTS"],
        vec!["A"],
        vec!["CAFÉ"],
        vec!["LONGERTHANTHEGRID"],
    ] {
        config.words = words.into_iter().map(str::to_string).collect();
        assert!(puzzles::generate(&config).is_err());
    }
    assert_eq!(
        puzzles::normalise("Sun-shine's Day").unwrap(),
        "SUNSHINESDAY"
    );
    let config = puzzles::WordSearchConfig {
        words: vec!["ABBA".into()],
        size: 8,
        ..Default::default()
    };
    let grid = vec![
        "ABBAZZZZ".into(),
        "ZZZZZZZZ".into(),
        "ABBAZZZZ".into(),
        "ZZZZZZZZ".into(),
        "ZZZZZZZZ".into(),
        "ZZZZZZZZ".into(),
        "ZZZZZZZZ".into(),
        "ZZZZZZZZ".into(),
    ];
    assert_eq!(puzzles::occurrences(&grid, "ABBA").len(), 2);
    assert!(puzzles::validate(
        &config,
        &grid,
        &[puzzles::Placement {
            word: "ABBA".into(),
            x: 0,
            y: 0,
            dx: 1,
            dy: 0
        }]
    )
    .is_err());
    let mut project = Project::default();
    let before = project.clone();
    let invalid = puzzles::WordSearchConfig {
        words: vec!["X".into()],
        ..Default::default()
    };
    assert!(puzzles::append_series(&mut project, &invalid, &Default::default(), 1, true).is_err());
    assert!(project == before);
}
#[test]
fn fifty_puzzles_with_answer_pages_round_trip_restyle_and_export_independently() {
    let root = std::env::current_dir().unwrap().join("qa/puzzle-proof");
    fs::create_dir_all(&root).unwrap();
    let mut project = Project::default();
    project.title = "Reproducible garden puzzle workbook".into();
    let config = puzzles::WordSearchConfig::default();
    puzzles::append_series(&mut project, &config, &Default::default(), 50, true).unwrap();
    assert_eq!(project.pages.len(), 100);
    assert_eq!(project.production.puzzles.len(), 50);
    let grid = project.production.puzzles[0].grid.clone();
    let placements = project.production.puzzles[0].placements.clone();
    let mut style = puzzles::PuzzleStyle::default();
    style.accent = rgb(90, 40, 105);
    style.grid_lines = true;
    style.family = FontFamily::DisplaySerif;
    puzzles::restyle(&mut project, 0, &style).unwrap();
    assert_eq!(project.production.puzzles[0].grid, grid);
    assert_eq!(project.production.puzzles[0].placements, placements);
    let package = root.join("garden-workbook.canvas");
    save_project_json(&project, &package).unwrap();
    let reopened = load_project_json(&package).unwrap();
    puzzles::validate_project(&reopened).unwrap();
    let report = run_export_preflight(&reopened, 300);
    assert!(
        !report.has_errors() && !report.has_overflows(),
        "{report:?}"
    );
    let proof = root.join("garden-puzzle.pdf");
    export_project_to_pdf(
        &reopened,
        &PdfExportOptions {
            page_selection: PageSelection::Range(0, 0),
            target_image_dpi: 96,
            ..Default::default()
        },
        &proof,
    )
    .unwrap();
    let answer = root.join("garden-answer.pdf");
    export_project_to_pdf(
        &reopened,
        &PdfExportOptions {
            page_selection: PageSelection::Range(50, 50),
            target_image_dpi: 96,
            ..Default::default()
        },
        &answer,
    )
    .unwrap();
    let full = root.join("garden-workbook.pdf");
    export_project_to_pdf(
        &reopened,
        &PdfExportOptions {
            target_image_dpi: 96,
            ..Default::default()
        },
        &full,
    )
    .unwrap();
    let info = Command::new("pdfinfo").arg(full).output().unwrap();
    assert!(String::from_utf8_lossy(&info.stdout)
        .lines()
        .any(|l| l.starts_with("Pages:") && l.split_whitespace().last() == Some("100")));
    for (pdf, name) in [(&proof, "puzzle"), (&answer, "answer")] {
        let render = Command::new("pdftoppm")
            .args(["-singlefile", "-scale-to", "1100", "-png"])
            .arg(pdf)
            .arg(root.join(name))
            .output()
            .unwrap();
        assert!(render.status.success());
    }
    fs::write(
        root.join("recipe.json"),
        serde_json::to_vec_pretty(&reopened.production.puzzles[0]).unwrap(),
    )
    .unwrap();
}
#[test]
fn modified_puzzles_block_checked_export_and_rebuild_preserves_added_artwork() {
    let mut project = Project::default();
    puzzles::append_series(
        &mut project,
        &Default::default(),
        &Default::default(),
        1,
        true,
    )
    .unwrap();
    let added = templates::make_stock_image_element(
        project.alloc_id(),
        &generate_builtin_stock_assets()[0],
        50.,
        800.,
    );
    let id = added.id;
    project.pages[0].elements.push(added);
    puzzles::validate_project(&project).unwrap();
    let letter = project.pages[0]
        .elements
        .iter_mut()
        .find(|e| e.name == "Puzzle letter")
        .unwrap();
    if let ElementKind::Text { content, .. } = &mut letter.kind {
        *content = "?".into();
    }
    assert!(puzzles::validate_project(&project).is_err());
    assert!(run_export_preflight(&project, 300).has_errors());
    puzzles::restyle(&mut project, 0, &Default::default()).unwrap();
    puzzles::validate_project(&project).unwrap();
    assert!(project.pages[0].elements.iter().any(|e| e.id == id));
    let original = project.clone();
    let mut invalid = puzzles::PuzzleStyle::default();
    invalid.title_pt = f32::NAN;
    assert!(puzzles::restyle(&mut project, 0, &invalid).is_err());
    assert!(project == original);
}

#[test]
fn puzzle_recipes_work_in_a_fresh_process_and_survive_template_insertion() {
    if let Some(path) = std::env::var_os("CANVAS_PUZZLE_LOAD") {
        let mut project = load_project_json(Path::new(&path)).unwrap();
        puzzles::validate_project(&project).unwrap();
        let recipe = &project.production.puzzles[0];
        let (grid, placements) = puzzles::generate(&recipe.config).unwrap();
        assert_eq!(grid, recipe.grid);
        assert_eq!(placements, recipe.placements);
        let mut style = recipe.style.clone();
        style.accent = rgb(70, 90, 120);
        puzzles::restyle(&mut project, 0, &style).unwrap();
        let output = Path::new(&path).with_file_name("recipient-edit.canvas");
        save_project_json(&project, &output).unwrap();
        assert!(!run_export_preflight(&load_project_json(&output).unwrap(), 300).has_errors());
        return;
    }
    let root = std::env::temp_dir().join(model::generate_document_id());
    fs::create_dir_all(&root).unwrap();
    let mut source = Project::default();
    puzzles::append_series(
        &mut source,
        &Default::default(),
        &Default::default(),
        2,
        true,
    )
    .unwrap();
    let path = root.join("puzzles.canvas");
    save_project_json(&source, &path).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "puzzle_tests::puzzle_recipes_work_in_a_fresh_process_and_survive_template_insertion",
        ])
        .env("CANVAS_PUZZLE_LOAD", path)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stdout)
    );
    let mut destination = Project::default();
    let copies = document_ops::fitted_pages(&mut destination, &source);
    destination.pages.extend(copies);
    assert_eq!(destination.production.puzzles.len(), 2);
    puzzles::validate_project(&destination).unwrap();
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn small_and_large_grids_export_without_clipping() {
    for size in [8, 24] {
        let mut project = Project::default();
        let config = puzzles::WordSearchConfig {
            size,
            words: vec!["TREE".into(), "LEAF".into(), "SEED".into()],
            difficulty: puzzles::Difficulty::Hard,
            ..Default::default()
        };
        puzzles::append_series(&mut project, &config, &Default::default(), 1, true).unwrap();
        let report = run_export_preflight(&project, 300);
        assert!(
            !report.has_errors() && !report.has_overflows(),
            "{report:?}"
        );
    }
}
