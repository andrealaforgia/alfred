; Python highlight query for Alfred editor

; Types
(type (identifier) @type)

; Function definitions
(function_definition
  name: (identifier) @function)

; Function calls
(call
  function: (identifier) @function)
(call
  function: (attribute
    attribute: (identifier) @function))

; Decorators
(decorator) @attribute

; Comments
(comment) @comment

; Strings
(string) @string
(interpolation) @string

; Numbers
(integer) @number
(float) @number
(true) @number
(false) @number
(none) @number

; Keywords
"and" @keyword
"as" @keyword
"assert" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"class" @keyword
"continue" @keyword
"def" @keyword
"del" @keyword
"elif" @keyword
"else" @keyword
"except" @keyword
"finally" @keyword
"for" @keyword
"from" @keyword
"global" @keyword
"if" @keyword
"import" @keyword
"in" @keyword
"is" @keyword
"lambda" @keyword
"nonlocal" @keyword
"not" @keyword
"or" @keyword
"pass" @keyword
"raise" @keyword
"return" @keyword
"try" @keyword
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
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator

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

; Parameters
(parameters (identifier) @variable)

; Properties
(attribute
  attribute: (identifier) @property)

; Class definitions
(class_definition
  name: (identifier) @type)

; Self
(identifier) @variable
