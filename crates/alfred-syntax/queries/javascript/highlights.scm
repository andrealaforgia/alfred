; JavaScript highlight query for Alfred editor

; Function definitions
(function_declaration
  name: (identifier) @function)

(method_definition
  name: (property_identifier) @function)

; Function calls
(call_expression
  function: (identifier) @function)
(call_expression
  function: (member_expression
    property: (property_identifier) @function))

; Comments
(comment) @comment

; Strings
(string) @string
(template_string) @string

; Numbers
(number) @number
(true) @number
(false) @number
(null) @number
(undefined) @number

; Keywords
"async" @keyword
"await" @keyword
"break" @keyword
"case" @keyword
"catch" @keyword
"class" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"delete" @keyword
"do" @keyword
"else" @keyword
"export" @keyword
"extends" @keyword
"finally" @keyword
"for" @keyword
"function" @keyword
"if" @keyword
"import" @keyword
"in" @keyword
"instanceof" @keyword
"let" @keyword
"new" @keyword
"of" @keyword
"return" @keyword
"static" @keyword
"switch" @keyword
"throw" @keyword
"try" @keyword
"typeof" @keyword
"var" @keyword
"void" @keyword
"while" @keyword
"with" @keyword
"yield" @keyword

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"=" @operator

; Punctuation
"(" @punctuation
")" @punctuation
"[" @punctuation
"]" @punctuation
"{" @punctuation
"}" @punctuation
":" @punctuation
"," @punctuation
"." @punctuation
";" @punctuation

; Properties
(property_identifier) @property

; Variables
(identifier) @variable
