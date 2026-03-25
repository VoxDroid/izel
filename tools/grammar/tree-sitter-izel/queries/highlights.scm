; Keywords
[
  "forge"
  "shape"
  "scroll"
  "ward"
  "draw"
  "given"
  "else"
  "while"
  "let"
  "give"
  "return"
] @keyword

; Builtin booleans
(bool) @constant.builtin

; Types and identifiers
(type_expr (identifier) @type)
(forge_decl (identifier) @function)
(call_expr (path (identifier) @function.call))
(field_decl (identifier) @property)

; Literals
(number) @number
(string) @string

; Comments
(line_comment) @comment
