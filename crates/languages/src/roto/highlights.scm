; Identifiers

(identifier) @variable

; Assume that uppercase names in paths are types
((identifier) @type
 (#match? @type "^[A-Z]"))

(optional_type) @type
(type_name) @type
(unit) @type.builtin
(never) @type.builtin
(record_type) @type

(type_name (path ((identifier) @type.builtin))
  (#match? @type.builtin "^(u8|u16|u32|u64|i8|i16|i32|i64|f32|f64|String|bool|Asn|IpAddr|Prefix)$"))

; Assume all-caps names are constants
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]+$'"))

; Function calls
; Unfortunately, we can't really distinguish them from methods if it's called
; on a path

(call_expression
  function: (path
    (identifier) @function .))

(call_expression
  function: (access_expression
    (identifier) @function.method .))

; Calling a function with an uppercase letter: it's an enum constructor
(call_expression
  function: (path
    (identifier) @constructor .)
  (#match? @constructor "^[A-Z]"))

; Function definitions

(function_item (identifier) @function)
(filtermap_item (identifier) @function)
(test_item (identifier) @function)

(line_comment) @comment

(parameter (identifier) @variable.parameter)

[
  "*"
  "="
  "=="
  "!="
  "&&"
  "|"
  "||"
  "-"
  "+"
  "/"
  ">"
  "<"
  ">="
  "<="
  "?"
  "!"
] @operator

[
  "("
  ")"
  "{"
  "}"
  "["
  "]"
] @punctuation.bracket

[
  ":"
  "."
  ","
  ";"
  "->"
] @punctuation.delimiter

[
  "accept"
  "reject"
  "return"
] @keyword.control.return

[
  "if"
  "else"
  "match"
] @keyword.control.conditional

"for" @keyword.control.repeat
"while" @keyword.control.repeat

[
  "filter"
  "filtermap"
  "fn"
  "test"
] @keyword.function

"let" @keyword.storage
"record" @keyword.storage.type
"enum" @keyword.storage.type

"import" @keyword.control.import

"in" @keyword
; "dep" @keyword
; "std" @keyword
; "super" @keyword
; "pkg" @keyword

(string_literal) @string
(escape_sequence) @constant.character.escape

(unit_literal) @constant.builtin
(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(ipv4_literal) @constant.numeric
(ipv6_literal) @constant.numeric
