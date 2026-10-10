; Basic highlighting using the bundled Tree-sitter grammar.
(comment) @comment
(multiline_comment) @comment
(line_string_literal) @string
(multi_line_string_literal) @string
(raw_string_literal) @string
(integer_literal) @number
(real_literal) @number
(hex_literal) @number
(oct_literal) @number
(bin_literal) @number
(boolean_literal) @boolean
(nil_literal) @constant
(type_identifier) @type
(simple_identifier) @variable
["actor" "as" "associatedtype" "async" "await" "break" "case" "class" "continue" "deinit" "do" "enum" "extension" "fallthrough" "false" "fileprivate" "for" "func" "guard" "if" "import" "in" "init" "inout" "internal" "is" "lazy" "let" "mutating" "nonmutating" "open" "operator" "override" "private" "protocol" "public" "repeat" "return" "self" "static" "struct" "subscript" "super" "switch" "true" "try" "typealias" "var" "weak" "while"] @keyword
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
