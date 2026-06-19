[
  "package"
  "uses"
  "class"
  "function"
  "var"
  "new"
] @keyword

[
  "private"
  "internal"
  "protected"
  "public"
  "static"
  "abstract"
  "override"
  "final"
  "transient"
] @modifier

[
  "+"
  "-"
  "?+"
  "?-"
  "="
  ":"
  "."
  ","
] @operator

[
  "("
  ")"
  "{"
  "}"
] @punctuation.bracket

(LINE_COMMENT) @comment
(COMMENT) @comment

(StringLiteral) @string

(gClass
  name: (id) @type)

(newExpr
  (id) @constructor)

(functionDefn
  (id) @function)

(parameterDeclaration
  (id) @variable.parameter)

(fieldDefn
  (id) @property)

(localVarStatement
  (id) @variable)

(type_identifier) @type

(usesStatement
  (id) @namespace)

(namespaceStatement
  (id) @namespace)
