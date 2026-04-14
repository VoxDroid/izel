module.exports = grammar({
  name: "izel",

  extras: ($) => [/[\s\uFEFF\u2060\u200B]/, $.line_comment],

  word: ($) => $.identifier,

  rules: {
    source_file: ($) => repeat($._item),

    _item: ($) =>
      choice(
        $.forge_decl,
        $.shape_decl,
        $.scroll_decl,
        $.ward_decl,
        $.draw_decl,
        $.statement,
      ),

    draw_decl: ($) => seq("draw", $.path, optional(";")),

    ward_decl: ($) =>
      seq(
        "ward",
        $.identifier,
        "{",
        repeat($._item),
        "}",
      ),

    shape_decl: ($) =>
      seq(
        "shape",
        $.identifier,
        "{",
        repeat($.field_decl),
        "}",
      ),

    field_decl: ($) => seq($.identifier, ":", $.type_expr, optional(",")),

    scroll_decl: ($) =>
      seq(
        "scroll",
        $.identifier,
        "{",
        repeat($.variant_decl),
        "}",
      ),

    variant_decl: ($) =>
      seq(
        $.identifier,
        optional(seq("(", commaSep1($.type_expr), ")")),
        optional(","),
      ),

    forge_decl: ($) =>
      seq(
        "forge",
        $.identifier,
        optional($.type_params),
        "(",
        optional(commaSep1($.param)),
        ")",
        optional(seq("->", $.type_expr)),
        optional($.effect_list),
        $.block,
      ),

    type_params: ($) => seq("<", commaSep1($.identifier), ">"),

    param: ($) => seq($.identifier, ":", $.type_expr),

    type_expr: ($) => choice($.path, $.optional_type),

    optional_type: ($) => seq("?", $.type_expr),

    effect_list: ($) => repeat1(seq("!", $.identifier)),

    statement: ($) =>
      choice(
        $.let_stmt,
        $.assign_stmt,
        $.expr_stmt,
        $.given_stmt,
        $.while_stmt,
        $.return_stmt,
      ),

    let_stmt: ($) =>
      seq(
        choice("let", "~"),
        $.identifier,
        optional(seq(":", $.type_expr)),
        "=",
        $.expr,
        optional(";"),
      ),

    assign_stmt: ($) => seq($.identifier, "=", $.expr, optional(";")),

    return_stmt: ($) =>
      prec.right(seq(choice("give", "return"), optional($.expr), optional(";"))),

    given_stmt: ($) =>
      seq(
        "given",
        $.expr,
        $.block,
        optional(seq("else", $.block)),
      ),

    while_stmt: ($) => seq("while", $.expr, $.block),

    expr_stmt: ($) => seq($.expr, optional(";")),

    expr: ($) =>
      choice(
        $.binary_expr,
        $.call_expr,
        $.path,
        $.number,
        $.string,
        $.bool,
        $.block,
      ),

    binary_expr: ($) =>
      prec.left(
        1,
        seq($.expr, choice("+", "-", "*", "/", "==", "and", "or", "|>"), $.expr),
      ),

    call_expr: ($) => prec(2, seq($.path, "(", optional(commaSep1($.expr)), ")")),

    block: ($) => seq("{", repeat($.statement), "}"),

    path: ($) => seq($.identifier, repeat(seq("::", $.identifier))),

    line_comment: (_$) => token(seq("//", /.*/)),

    identifier: (_$) => /[A-Za-z_][A-Za-z0-9_]*/,

    number: (_$) => /[0-9][0-9_]*/,

    string: (_$) => /"([^"\\]|\\.)*"/,

    bool: (_$) => choice("true", "false"),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
