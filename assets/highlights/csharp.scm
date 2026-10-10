; Basic highlighting using the bundled Tree-sitter grammar.
(comment) @comment
(character_literal) @string
(string_literal) @string
(raw_string_literal) @string
(verbatim_string_literal) @string
(interpolated_string_expression) @string
(escape_sequence) @string.escape
(integer_literal) @number
(real_literal) @number
(boolean_literal) @boolean
(null_literal) @constant
(predefined_type) @type
(identifier) @variable
["abstract" "as" "async" "await" "base" "break" "case" "catch" "class" "const" "continue" "default" "delegate" "do" "else" "enum" "event" "explicit" "extern" "false" "finally" "fixed" "for" "foreach" "goto" "if" "implicit" "in" "interface" "internal" "is" "lock" "namespace" "new" "operator" "out" "override" "params" "private" "protected" "public" "readonly" "record" "ref" "return" "sealed" "sizeof" "stackalloc" "static" "struct" "switch" "this" "throw" "true" "try" "typeof" "unchecked" "unsafe" "using" "virtual" "volatile" "while" "yield"] @keyword
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
