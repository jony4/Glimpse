; Basic highlighting using the bundled Tree-sitter grammar.
(comment) @comment
(string_value) @string
(int_value) @number
(float_value) @number
(boolean_value) @boolean
(named_type) @type
(variable) @variable
(name) @property
["query" "mutation" "subscription" "fragment" "on" "schema" "scalar" "type" "interface" "union" "enum" "input" "extend" "implements" "directive" "repeatable" "true" "false"] @keyword
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
