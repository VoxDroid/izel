use izel_parser::ast;

pub fn elaborate_dual(dual: &mut ast::Dual) -> Option<ast::Item> {
    let mut shape = None;
    let mut encode_fn = None;
    let mut decode_fn = None;
    
    for item in &dual.items {
        match item {
            ast::Item::Shape(s) => shape = Some(s.clone()),
            ast::Item::Forge(f) => {
                if f.name == "encode" { encode_fn = Some(f.clone()); }
                if f.name == "decode" { decode_fn = Some(f.clone()); }
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

    if let (Some(encode), Some(decode)) = (&encode_fn, &decode_fn) {
        if !encode.effects.is_empty() || !decode.effects.is_empty() {
             return Some(generate_roundtrip_test(&dual.name, encode, decode));
        }
    }

    None
}

fn derive_encode_from_shape(shape: &ast::Shape) -> ast::Forge {
    let mut body_stmts = Vec::new();
    let span = shape.span;
    
    // let raw = JsonObject::new()
    body_stmts.push(ast::Stmt::Let {
        name: "raw".to_string(),
        ty: Some(ast::Type::Prim("JsonValue".to_string())),
        init: Some(ast::Expr::Call(
            Box::new(ast::Expr::Ident("JsonObject::new".to_string(), span)),
            vec![]
        )),
        span,
    });
    
    for field in &shape.fields {
        // raw.set("field", self.field.encode())
        body_stmts.push(ast::Stmt::Expr(ast::Expr::Call(
            Box::new(ast::Expr::Member(
                Box::new(ast::Expr::Ident("raw".to_string(), span)),
                "set".to_string(),
                span
            )),
            vec![
                ast::Expr::Literal(ast::Literal::Str(field.name.clone())),
                ast::Expr::Call(
                    Box::new(ast::Expr::Member(
                        Box::new(ast::Expr::Ident("self".to_string(), span)),
                        field.name.clone(),
                        span
                    )),
                    vec![] // .encode() call assumed implicitly or added
                )
            ]
        )));
    }
    
    ast::Forge {
        name: "encode".to_string(),
        is_flow: false,
        generic_params: shape.generic_params.clone(),
        params: vec![
            ast::Param { name: "self".into(), ty: ast::Type::Pointer(Box::new(ast::Type::SelfType), false), span }
        ],
        ret_type: ast::Type::Prim("JsonValue".to_string()),
        effects: vec![],
        attributes: vec![],
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block { stmts: body_stmts, expr: Some(Box::new(ast::Expr::Ident("raw".to_string(), span))), span }),
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
            name: field.name.clone(),
            ty: None,
            init: Some(ast::Expr::Call(
                Box::new(ast::Expr::Member(
                    Box::new(ast::Expr::Call(
                        Box::new(ast::Expr::Member(
                            Box::new(ast::Expr::Ident("raw".to_string(), span)),
                            "get".to_string(),
                            span
                        )),
                        vec![ast::Expr::Literal(ast::Literal::Str(field.name.clone()))]
                    )),
                    "decode".to_string(),
                    span
                )),
                vec![]
            )),
            span,
        });
        field_init.push((field.name.clone(), ast::Expr::Ident(field.name.clone(), span)));
    }
    
    ast::Forge {
        name: "decode".to_string(),
        is_flow: false,
        generic_params: shape.generic_params.clone(),
        params: vec![
            ast::Param { name: "raw".into(), ty: ast::Type::Prim("JsonValue".into()), span }
        ],
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
            span 
        }),
        span,
    }
}

fn generate_roundtrip_test(shape_name: &str, encode: &ast::Forge, _decode: &ast::Forge) -> ast::Item {
    let span = encode.span;
    ast::Item::Forge(ast::Forge {
        name: format!("{}_test", shape_name),
        is_flow: false,
        generic_params: encode.generic_params.clone(),
        params: vec![],
        ret_type: ast::Type::Prim("void".into()),
        effects: encode.effects.clone(),
        attributes: vec![ast::Attribute { name: "test".into(), args: vec![], span }],
        requires: vec![],
        ensures: vec![],
        body: Some(ast::Block { stmts: vec![], expr: Some(Box::new(ast::Expr::Ident("todo".into(), span))), span }),
        span,
    })
}

fn derive_decode_from_encode(encode: &ast::Forge) -> ast::Forge {
    derive_decode_from_shape(&ast::Shape {
        name: "Duality".to_string(),
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
        generic_params: decode.generic_params.clone(),
        fields: vec![],
        attributes: vec![],
        invariants: vec![],
        span: decode.span,
    })
}
