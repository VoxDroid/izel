use izel_query::{Database, QueryContext};

#[test]
fn database_set_get_roundtrip_for_typed_values() {
    let mut db = Database::new();
    db.set("answer".to_string(), 42_u32);

    let value = db.get::<u32>("answer").expect("typed value should exist");
    assert_eq!(*value, 42);
}

#[test]
fn database_get_with_wrong_type_returns_none() {
    let mut db = Database::new();
    db.set("answer".to_string(), 42_u32);

    assert!(db.get::<String>("answer").is_none());
}

#[test]
fn database_implements_query_context_downcast() {
    let db = Database::new();
    let ctx: &dyn QueryContext = &db;
    assert!(ctx.as_any().downcast_ref::<Database>().is_some());
}

#[test]
fn database_default_and_missing_key_are_handled() {
    let mut db = Database::default();
    assert!(db.get::<u32>("missing").is_none());

    db.set("present".to_string(), 9_u32);
    let got = db.get::<u32>("present").expect("value should be present");
    assert_eq!(*got, 9);
}
