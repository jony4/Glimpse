; Basic highlighting using the bundled Tree-sitter grammar.
(comment) @comment
(string) @string
(int_lit) @number
(float_lit) @number
(field_number) @number
(type) @type
(message_name) @type
(enum_name) @type
(service_name) @type
(rpc_name) @function
(identifier) @variable
["syntax" "import" "weak" "public" "package" "option" "message" "enum" "service" "rpc" "returns" "stream" "optional" "repeated" "required" "oneof" "map" "reserved" "extensions" "to" "max"] @keyword
["(" ")" "[" "]" "{" "}"] @punctuation.bracket
