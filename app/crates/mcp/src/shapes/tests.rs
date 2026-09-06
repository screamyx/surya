//! Tests for the six built-in card shapes.

use super::*;
use std::collections::HashMap;

fn root_of(components: &[Value]) -> &Value {
    components
        .iter()
        .find(|c| c["id"] == "root")
        .expect("every surface has a root")
}

/// Every id a component points at must exist, or the renderer draws a
/// placeholder forever.
fn assert_links_resolve(components: &[Value]) {
    let ids: Vec<&str> = components
        .iter()
        .filter_map(|c| c["id"].as_str())
        .collect();
    for component in components {
        if let Some(child) = component["child"].as_str() {
            assert!(ids.contains(&child), "dangling child {child}");
        }
        if let Some(children) = component["children"].as_array() {
            for child in children {
                let child = child.as_str().expect("child ids are strings");
                assert!(ids.contains(&child), "dangling child {child}");
            }
        }
    }
}

#[test]
fn every_built_in_shape_expands_to_a_linked_tree() {
    let cards = vec![
        json!({"shape":"record","title":"A","subtitle":"b","image":"http://x/y.png",
               "badges":["ok"],"fields":[{"label":"Price","value":"12"}],
               "actions":[{"label":"Open","event":"open"}]}),
        json!({"shape":"table","title":"T","columns":["A","B"],"rows":[["1","2"],["3","4"]]}),
        json!({"shape":"form","title":"F","fields":[
                 {"label":"Name","type":"text","value":"x","key":"name"},
                 {"label":"Who","type":"select","value":"a","options":["a","b"]},
                 {"label":"When","type":"date","value":"2026-09-06"}],"submit":"Save"}),
        json!({"shape":"approval","title":"A","summary":"s","code":"rm -rf nothing"}),
        json!({"shape":"diff-summary","title":"D",
               "files":[{"path":"a.rs","added":3,"removed":1}],"note":"n"}),
        json!({"shape":"metric","title":"M","value":"38","delta":"+12","series":[1,2,3]}),
    ];
    let mut expanded = 0;
    for card in &cards {
        let (components, _) = expand(card).expect("built-in shapes expand");
        assert_eq!(root_of(&components)["component"], "Card");
        assert_links_resolve(&components);
        expanded += 1;
    }
    assert_eq!((cards.len(), expanded), (6, 6), "asked=6 expanded={expanded}");
}

#[test]
fn table_rows_become_one_row_component_each() {
    let (components, _) = expand(&json!({
        "shape": "table", "columns": ["A", "B"], "rows": [["1", "2"], ["3", "4"]]
    }))
    .unwrap();
    let rows = components
        .iter()
        .filter(|c| c["component"] == "Row")
        .count();
    // one header row plus one per data row
    assert_eq!(rows, 3);
    let texts: Vec<&str> = components
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect();
    assert!(texts.contains(&"3"), "row cells are rendered: {texts:?}");

    // Every cell shares the row, or the renderer wraps it `flex_none` and
    // the columns drift row by row (E2E-CARD-03).
    let by_id: HashMap<&str, &Value> = components
        .iter()
        .filter_map(|c| Some((c["id"].as_str()?, c)))
        .collect();
    let mut asked = 0;
    let mut weighted = 0;
    for row in components.iter().filter(|c| c["component"] == "Row") {
        for child in row["children"].as_array().expect("a row has children") {
            let id = child.as_str().expect("child ids are strings");
            let cell = by_id.get(id).expect("a row child resolves");
            asked += 1;
            if cell["weight"] == json!(1) {
                weighted += 1;
            }
        }
    }
    assert_eq!((asked, weighted), (6, 6), "asked={asked} weighted={weighted}");
}

/// The hand-written reference table already had columns; the short form
/// did not. Read the fixture rather than restating its number, so the two
/// cannot drift apart in silence.
#[test]
fn short_form_cells_carry_the_same_weight_as_the_reference_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../a2ui/fixtures/02-table.json");
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the fixture is readable"))
            .expect("the fixture is JSON");
    let fixture_weights: Vec<&Value> = fixture
        .as_array()
        .expect("the fixture is a message array")
        .iter()
        .filter_map(|m| m["updateComponents"]["components"].as_array())
        .flatten()
        .filter(|c| c["component"] == "Text")
        .filter_map(|c| c.get("weight"))
        .collect();
    assert!(
        !fixture_weights.is_empty(),
        "the fixture still weights its cells; if it stopped, this test is the wrong shape"
    );
    let reference = fixture_weights[0];
    assert!(
        fixture_weights.iter().all(|w| *w == reference),
        "the fixture uses one weight for every cell: {fixture_weights:?}"
    );

    let (components, _) = expand(&json!({
        "shape": "table", "columns": ["Lead"], "rows": [["Ada"]]
    }))
    .unwrap();
    let mut asked = 0;
    let mut matched = 0;
    for cell in components.iter().filter(|c| c["component"] == "Text") {
        asked += 1;
        if cell["weight"] == *reference {
            matched += 1;
        }
    }
    assert_eq!(
        (asked, matched),
        (2, 2),
        "asked={asked} matched={matched} against fixture weight {reference}"
    );
}

#[test]
fn form_seeds_the_data_model_under_form() {
    let (_, data) = expand(&json!({
        "shape": "form",
        "fields": [{"label": "Name", "type": "text", "value": "Ada", "key": "name"}],
        "submit": "Save"
    }))
    .unwrap();
    assert_eq!(data["form"]["name"], "Ada");
}

#[test]
fn metric_series_rides_the_data_model() {
    let (_, data) = expand(&json!({"shape":"metric","value":"1","series":[1,2,3]})).unwrap();
    assert_eq!(data["series"], json!([1, 2, 3]));
}

#[test]
fn an_unknown_shape_names_the_ones_that_exist() {
    let error = expand(&json!({"shape": "chart"})).unwrap_err();
    assert!(error.contains("record"), "{error}");
    assert!(error.contains("list_cards"), "{error}");
}
