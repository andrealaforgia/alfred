; Rust highlight query for Alfred editor
; Simplified from tree-sitter-rust official query (no predicates)

; Types
(type_identifier) @type
(primitive_type) @type

; Properties
(field_identifier) @property

; Function calls
(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function))
(call_expression
  function: (scoped_identifier
    "::"
    name: (identifier) @function))

(generic_function
  function: (identifier) @function)
(generic_function
  function: (scoped_identifier
    name: (identifier) @function))

(macro_invocation
  macro: (identifier) @function
  "!" @function)

; Function definitions
(function_item (identifier) @function)
(function_signature_item (identifier) @function)

; Comments
(line_comment) @comment
(block_comment) @comment

; Brackets
"(" @punctuation
")" @punctuation
"[" @punctuation
"]" @punctuation
"{" @punctuation
"}" @punctuation

; Delimiters
"::" @punctuation
":" @punctuation
"." @punctuation
"," @punctuation
";" @punctuation

; Parameters
(parameter (identifier) @variable)

; Keywords
"as" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"dyn" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"fn" @keyword
"for" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"loop" @keyword
"macro_rules!" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"pub" @keyword
"ref" @keyword
"return" @keyword
"static" @keyword
"struct" @keyword
"trait" @keyword
"type" @keyword
"union" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword
"yield" @keyword
(crate) @keyword
(mutable_specifier) @keyword
(self) @keyword
(super) @keyword

; Strings
(char_literal) @string
(string_literal) @string
(raw_string_literal) @string

; Numbers / constants
(boolean_literal) @number
(integer_literal) @number
(float_literal) @number

; Escape sequences
(escape_sequence) @string

; Attributes
(attribute_item) @attribute
(inner_attribute_item) @attribute

; Operators
"*" @operator
"&" @operator
"=" @operator
"!" @operator
"<" @operator
">" @operator
"-" @operator
"+" @operator
"%" @operator
"/" @operator
