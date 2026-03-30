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
