use izel_parser::ast;

pub fn elaborate_dual(dual: &mut ast::Dual) -> Option<ast::Item> {
    let mut shape = None;
    let mut encode_fn = None;
    let mut decode_fn = None;

    for item in &dual.items {
        match item {
            ast::Item::Shape(s) => shape = Some(s.clone()),
            ast::Item::Forge(f) => {
                if f.name == "encode" {
                    encode_fn = Some(f.clone());
                }
                if f.name == "decode" {
                    decode_fn = Some(f.clone());
                }
            }
            _ => {}
        }
    }

    if let (Some(s), None, None) = (&shape, &encode_fn, &decode_fn) {
        let encode = derive_encode_from_shape(s);
        let decode = derive_decode_from_shape(s);
        dual.items.push(ast::Item::Forge(encode.clone()));
        dual.items.push(ast::Item::Forge(decode.clone()));
        encode_fn = Some(encode);
        decode_fn = Some(decode);
    } else if let (Some(encode), None) = (&encode_fn, &decode_fn) {
        let decode = derive_decode_from_encode(encode);
        dual.items.push(ast::Item::Forge(decode.clone()));
        decode_fn = Some(decode);
    } else if let (None, Some(decode)) = (&encode_fn, &decode_fn) {
        let encode = derive_encode_from_decode(decode);
        dual.items.push(ast::Item::Forge(encode.clone()));
        encode_fn = Some(encode);
    }

    match (&encode_fn, &decode_fn) {
        (Some(encode), Some(decode))
            if !encode.effects.is_empty() || !decode.effects.is_empty() =>
        {
            return Some(generate_roundtrip_test(&dual.name, encode, decode));
        }
        _ => {}
    }

    None
}

fn derive_encode_from_shape(shape: &ast::Shape) -> ast::Forge {
    let mut body_stmts = Vec::new();
    let span = shape.span;

    // let raw = JsonObject::new()
    body_stmts.push(ast::Stmt::Let {
        pat: ast::Pattern::Ident("raw".to_string(), false, span),
        ty: Some(ast::Type::Prim("JsonValue".to_string())),
        init: Some(ast::Expr::Call(
            Box::new(ast::Expr::Ident("JsonObject::new".to_string(), span)),
            vec![],
        )),
        span,
    });

    for field in &shape.fields {
        // raw.set("field", self.field.encode())
        body_stmts.push(ast::Stmt::Expr(ast::Expr::Call(
            Box::new(ast::Expr::Member(
                Box::new(ast::Expr::Ident("raw".to_string(), span)),
                "set".to_string(),
                span,
            )),
            vec![
                ast::Arg {
                    label: None,
                    value: ast::Expr::Literal(ast::Literal::Str(field.name.clone())),
                    span,
                },
                ast::Arg {
                    label: None,
                    value: ast::Expr::Call(
                        Box::new(ast::Expr::Member(
                            Box::new(ast::Expr::Ident("self".to_string(), span)),
                            field.name.clone(),
                            span,
                        )),
                        vec![],
                    ),
                    span,
                },
            ],
        )));
    }

    ast::Forge {
        name: "encode".to_string(),
        name_span: span,
        visibility: shape.visibility.clone(),
        is_flow: false,
        generic_params: shape.generic_params.clone(),
        params: vec![ast::Param {
            name: "self".into(),
            ty: ast::Type::Pointer(Box::new(ast::Type::SelfType), false),
            default_value: None,
            is_variadic: false,
            span,
        }],
        ret_type: ast::Type::Prim("JsonValue".to_string()),
        effects: vec![],
        attributes: vec![],
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block {
            stmts: body_stmts,
            expr: Some(Box::new(ast::Expr::Ident("raw".to_string(), span))),
            span,
        }),
        span,
    }
}

fn derive_decode_from_shape(shape: &ast::Shape) -> ast::Forge {
    let mut body_stmts = Vec::new();
    let span = shape.span;
    let mut field_init = Vec::new();

    for field in &shape.fields {
        // let field = raw.get("field").decode()
        body_stmts.push(ast::Stmt::Let {
            pat: ast::Pattern::Ident(field.name.clone(), false, span),
            ty: None,
            init: Some(ast::Expr::Call(
                Box::new(ast::Expr::Member(
                    Box::new(ast::Expr::Call(
                        Box::new(ast::Expr::Member(
                            Box::new(ast::Expr::Ident("raw".to_string(), span)),
                            "get".to_string(),
                            span,
                        )),
                        vec![ast::Arg {
                            label: None,
                            value: ast::Expr::Literal(ast::Literal::Str(field.name.clone())),
                            span,
                        }],
                    )),
                    "decode".to_string(),
                    span,
                )),
                vec![],
            )),
            span,
        });
        field_init.push((
            field.name.clone(),
            ast::Expr::Ident(field.name.clone(), span),
        ));
    }

    ast::Forge {
        name: "decode".to_string(),
        name_span: span,
        visibility: shape.visibility.clone(),
        is_flow: false,
        generic_params: shape.generic_params.clone(),
        params: vec![ast::Param {
            name: "raw".into(),
            ty: ast::Type::Prim("JsonValue".into()),
            default_value: None,
            is_variadic: false,
            span,
        }],
        ret_type: ast::Type::Cascade(Box::new(ast::Type::SelfType)),
        effects: vec![],
        attributes: vec![],
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block {
            stmts: body_stmts,
            expr: Some(Box::new(ast::Expr::StructLiteral {
                path: ast::Type::SelfType,
                fields: field_init,
            })),
            span,
        }),
        span,
    }
}

fn generate_roundtrip_test(
    shape_name: &str,
    encode: &ast::Forge,
    decode: &ast::Forge,
) -> ast::Item {
    let span = encode.span;
    let mut effects = encode.effects.clone();
    for eff in &decode.effects {
        if !effects.contains(eff) {
            effects.push(eff.clone());
        }
    }

    ast::Item::Forge(ast::Forge {
        name: format!("{}_test", shape_name),
        name_span: span,
        visibility: ast::Visibility::Hidden,
        is_flow: false,
        generic_params: encode.generic_params.clone(),
        params: vec![],
        ret_type: ast::Type::Prim("void".into()),
        effects,
        attributes: vec![ast::Attribute {
            name: "test".into(),
            args: vec![],
            span,
        }],
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block {
            stmts: vec![],
            expr: Some(Box::new(ast::Expr::Ident("todo".into(), span))),
            span,
        }),
        span,
    })
}

fn derive_decode_from_encode(encode: &ast::Forge) -> ast::Forge {
    derive_decode_from_shape(&ast::Shape {
        name: "Duality".to_string(),
        visibility: ast::Visibility::Hidden,
        generic_params: encode.generic_params.clone(),
        fields: vec![],
        attributes: vec![],
        invariants: vec![],
        span: encode.span,
    })
}

fn derive_encode_from_decode(decode: &ast::Forge) -> ast::Forge {
    derive_encode_from_shape(&ast::Shape {
        name: "Duality".to_string(),
        visibility: ast::Visibility::Hidden,
        generic_params: decode.generic_params.clone(),
        fields: vec![],
        attributes: vec![],
        invariants: vec![],
        span: decode.span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_span::Span;

    fn span() -> Span {
        Span::dummy()
    }

    fn mk_forge(name: &str, effects: &[&str]) -> ast::Forge {
        ast::Forge {
            name: name.to_string(),
            name_span: span(),
            visibility: ast::Visibility::Hidden,
            is_flow: false,
            generic_params: vec![],
            params: vec![],
            ret_type: ast::Type::SelfType,
            effects: effects.iter().map(|eff| (*eff).to_string()).collect(),
            attributes: vec![],
            requires: vec![],
            ensures: vec![],
            body: None,
            span: span(),
        }
    }

    fn mk_shape(name: &str) -> ast::Shape {
        ast::Shape {
            name: name.to_string(),
            visibility: ast::Visibility::Hidden,
            generic_params: vec![],
            fields: vec![ast::Field {
                name: "v".to_string(),
                visibility: ast::Visibility::Hidden,
                ty: ast::Type::Prim("i32".to_string()),
                span: span(),
            }],
            attributes: vec![],
            invariants: vec![],
            span: span(),
        }
    }

    fn mk_dual(name: &str, items: Vec<ast::Item>) -> ast::Dual {
        ast::Dual {
            name: name.to_string(),
            visibility: ast::Visibility::Hidden,
            generic_params: vec![],
            items,
            attributes: vec![],
            span: span(),
        }
    }

    fn find_forge<'a>(items: &'a [ast::Item], name: &str) -> Option<&'a ast::Forge> {
        items.iter().find_map(|item| match item {
            ast::Item::Forge(forge) if forge.name == name => Some(forge),
            _ => None,
        })
    }

    #[test]
    fn elaborate_dual_encode_only_generates_decode_and_ignores_non_forge_items() {
        let mut dual = mk_dual(
            "Codec",
            vec![
                ast::Item::Scroll(ast::Scroll {
                    name: "Kind".to_string(),
                    visibility: ast::Visibility::Hidden,
                    variants: vec![],
                    attributes: vec![],
                    span: span(),
                }),
                ast::Item::Forge(mk_forge("encode", &[])),
            ],
        );

        let generated = elaborate_dual(&mut dual);

        assert!(generated.is_none());
        assert!(find_forge(&dual.items, "encode").is_some());
        assert!(find_forge(&dual.items, "decode").is_some());
    }

    #[test]
    fn elaborate_dual_decode_only_generates_encode_and_roundtrip_test_when_effectful() {
        let mut dual = mk_dual("Codec", vec![ast::Item::Forge(mk_forge("decode", &["io"]))]);

        let generated = elaborate_dual(&mut dual);

        assert!(find_forge(&dual.items, "encode").is_some());
        assert!(find_forge(&dual.items, "decode").is_some());

        assert!(matches!(generated, Some(ast::Item::Forge(_))));
        let mut roundtrip_opt = None;
        if let Some(ast::Item::Forge(roundtrip)) = generated {
            roundtrip_opt = Some(roundtrip);
        }
        let roundtrip = roundtrip_opt.expect("expected generated forge item");
        assert_eq!(roundtrip.name, "Codec_test");
        assert!(roundtrip.effects.iter().any(|eff| eff == "io"));
        assert!(roundtrip.attributes.iter().any(|attr| attr.name == "test"));
    }

    #[test]
    fn elaborate_dual_shape_only_generates_encode_and_decode_without_roundtrip() {
        let mut dual = mk_dual("Point", vec![ast::Item::Shape(mk_shape("Point"))]);

        let generated = elaborate_dual(&mut dual);

        assert!(generated.is_none());
        assert_eq!(dual.items.len(), 3);
        assert!(find_forge(&dual.items, "encode").is_some());
        assert!(find_forge(&dual.items, "decode").is_some());
    }
}
