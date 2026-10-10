; Basic highlighting using the bundled Tree-sitter grammar.
(line_comment) @comment
(bracket_comment) @comment
(quoted_argument) @string
(bracket_argument) @string
(variable_ref) @variable
(identifier) @function
["(" ")" "{" "}"] @punctuation.bracket
