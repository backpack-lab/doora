# doora Query Guide

A comprehensive reference for writing S-expression queries against every supported language. Each pattern is tested against the real Tree-sitter grammars used by `doora`.

---

## Table of Contents

- [Query Syntax Primer](#query-syntax-primer)
- [How to Discover Node Types](#how-to-discover-node-types)
- [Rust](#rust)
- [Python](#python)
- [JavaScript](#javascript)
- [TypeScript](#typescript)
- [Go](#go)
- [C](#c)
- [C++](#c-1)
- [Cross-Language Patterns](#cross-language-patterns)
- [Predicate Reference](#predicate-reference)
- [Advanced Patterns](#advanced-patterns)
- [Troubleshooting](#troubleshooting)

---

## Query Syntax Primer

An S-expression query mirrors the shape of the syntax tree. It matches any node of the given type that satisfies all of its children constraints.

```
(node_type field_name: (child_type) @capture_name)
```

- `node_type` — the Tree-sitter node kind (e.g. `function_item`, `identifier`)
- `field_name:` — optional named field that constrains which child to match
- `@capture_name` — tags the matched node for extraction in output
- Predicates like `(#eq? @cap "value")` filter captures by text content

**Run a query:**

```bash
doora -q 'YOUR_QUERY_HERE' -p ./src --lang rust
```

**Run multiple queries in one pass:**

```bash
doora \
  -q '(function_item name: (identifier) @fn_name)' \
  -q '(struct_item name: (type_identifier) @struct_name)' \
  -p ./src
```

---

## How to Discover Node Types

Use the interactive TUI to explore the CST of any file:

```bash
doora -q '(function_item)' -p ./src --tui
```

Press `Tab` to focus the AST pane. The right pane shows the full CST with node kinds, field names, and byte positions. Expand and collapse nodes with `Enter`.

Alternatively, the [Tree-sitter playground](https://tree-sitter.github.io/tree-sitter/playground) lets you paste source code and see its parse tree interactively.

---

## Rust

### Function definitions

**All function definitions:**

```bash
doora -q '(function_item name: (identifier) @fn_name)' -p ./src
```

**A specific function by exact name:**

```bash
doora \
  -q '(function_item name: (identifier) @fn (#eq? @fn "authenticate"))' \
  -p ./src
```

**Functions matching a naming pattern:**

```bash
# All functions starting with handle_
doora \
  -q '(function_item name: (identifier) @fn (#match? @fn "^handle_"))' \
  -p ./src

# All test functions
doora \
  -q '(function_item name: (identifier) @fn (#match? @fn "^test_"))' \
  -p ./src

# All get/set/update functions
doora \
  -q '(function_item name: (identifier) @fn (#match? @fn "^(get|set|update)_"))' \
  -p ./src
```

**Public functions only:**

```bash
doora \
  -q '(function_item
        (visibility_modifier) @vis
        name: (identifier) @fn_name)' \
  -p ./src
```

**Async functions:**

```bash
doora \
  -q '(function_item
        (function_modifiers "async")
        name: (identifier) @fn_name)' \
  -p ./src
```

**Unsafe functions:**

```bash
doora \
  -q '(function_item
        (function_modifiers "unsafe")
        name: (identifier) @fn_name)' \
  -p ./src
```

**Functions with a specific return type:**

```bash
# Functions returning Result
doora \
  -q '(function_item
        name: (identifier) @fn_name
        return_type: (generic_type type: (type_identifier) @ret (#eq? @ret "Result")))' \
  -p ./src

# Functions returning bool
doora \
  -q '(function_item
        name: (identifier) @fn_name
        return_type: (primitive_type) @ret (#eq? @ret "bool"))' \
  -p ./src
```

**Functions with exactly zero parameters:**

```bash
doora \
  -q '(function_item
        name: (identifier) @fn_name
        parameters: (parameters . ")"))' \
  -p ./src
```

**Functions with a specific argument count:**

Tree-sitter does not expose a built-in child-count predicate. The reliable approach is to query for the parameter pattern explicitly:

```bash
# Functions with exactly one parameter (besides self)
doora \
  -q '(function_item
        name: (identifier) @fn_name
        parameters: (parameters
          . (parameter) .
          ")"))' \
  -p ./src

# Functions with exactly two parameters
doora \
  -q '(function_item
        name: (identifier) @fn_name
        parameters: (parameters
          . (parameter) . (parameter) .
          ")"))' \
  -p ./src
```

The `. node .` anchor syntax means "exactly this node with nothing between the neighbors."

**Methods (functions inside impl blocks):**

```bash
# All methods in any impl block
doora \
  -q '(impl_item
        body: (declaration_list
          (function_item name: (identifier) @method_name)))' \
  -p ./src

# Methods on a specific type
doora \
  -q '(impl_item
        type: (type_identifier) @t (#eq? @t "Config")
        body: (declaration_list
          (function_item name: (identifier) @method_name)))' \
  -p ./src
```

### Struct definitions

**All structs:**

```bash
doora -q '(struct_item name: (type_identifier) @struct_name)' -p ./src
```

**A specific struct:**

```bash
doora \
  -q '(struct_item name: (type_identifier) @s (#eq? @s "AppConfig"))' \
  -p ./src
```

**Structs with a specific field:**

```bash
doora \
  -q '(struct_item
        name: (type_identifier) @struct_name
        body: (field_declaration_list
          (field_declaration
            name: (field_identifier) @field (#eq? @field "timeout"))))' \
  -p ./src
```

**Struct fields of a specific type:**

```bash
# Fields of type String
doora \
  -q '(field_declaration
        name: (field_identifier) @field_name
        type: (type_identifier) @t (#eq? @t "String"))' \
  -p ./src

# Fields of type Option<T>
doora \
  -q '(field_declaration
        name: (field_identifier) @field_name
        type: (generic_type
          type: (type_identifier) @t (#eq? @t "Option")))' \
  -p ./src

# Fields of any Vec type
doora \
  -q '(field_declaration
        name: (field_identifier) @field_name
        type: (generic_type
          type: (type_identifier) @t (#eq? @t "Vec")))' \
  -p ./src
```

### Enum definitions

**All enums:**

```bash
doora -q '(enum_item name: (type_identifier) @enum_name)' -p ./src
```

**Enum variants:**

```bash
doora \
  -q '(enum_item
        name: (type_identifier) @enum_name
        body: (enum_variant_list
          (enum_variant name: (identifier) @variant_name)))' \
  -p ./src
```

### Trait definitions and implementations

**All trait definitions:**

```bash
doora -q '(trait_item name: (type_identifier) @trait_name)' -p ./src
```

**All trait implementations (impl Trait for Type):**

```bash
doora \
  -q '(impl_item
        trait: (type_identifier) @trait_name
        type: (type_identifier) @type_name)' \
  -p ./src
```

**Implementations of a specific trait:**

```bash
doora \
  -q '(impl_item
        trait: (type_identifier) @t (#eq? @t "Display")
        type: (type_identifier) @type_name)' \
  -p ./src
```

### Type aliases

**All type aliases:**

```bash
doora -q '(type_item name: (type_identifier) @alias_name)' -p ./src
```

**A specific type alias:**

```bash
doora \
  -q '(type_item name: (type_identifier) @t (#eq? @t "Result"))' \
  -p ./src
```

### Constants and statics

**All constants:**

```bash
doora -q '(const_item name: (identifier) @const_name)' -p ./src
```

**All static items:**

```bash
doora -q '(static_item name: (identifier) @static_name)' -p ./src
```

### Macros and attributes

**All macro invocations:**

```bash
doora -q '(macro_invocation macro: (identifier) @macro_name)' -p ./src
```

**A specific macro:**

```bash
# All println! calls
doora \
  -q '(macro_invocation macro: (identifier) @m (#eq? @m "println"))' \
  -p ./src

# All todo! calls
doora \
  -q '(macro_invocation macro: (identifier) @m (#eq? @m "todo"))' \
  -p ./src

# All panic! calls
doora \
  -q '(macro_invocation macro: (identifier) @m (#eq? @m "panic"))' \
  -p ./src

# All unimplemented! calls
doora \
  -q '(macro_invocation macro: (identifier) @m (#eq? @m "unimplemented"))' \
  -p ./src
```

**All derive attributes:**

```bash
doora -q '(attribute (identifier) @attr (#eq? @attr "derive"))' -p ./src
```

**Items with a specific attribute:**

```bash
# All #[test] functions
doora \
  -q '(attribute_item (attribute (identifier) @attr (#eq? @attr "test")))' \
  -p ./src

# All #[derive(Debug)] items
doora \
  -q '(attribute_item
        (attribute
          (identifier) @attr (#eq? @attr "derive")
          arguments: (token_tree (identifier) @derived (#eq? @derived "Debug"))))' \
  -p ./src
```

### Call sites and method calls

**All .unwrap() calls:**

```bash
doora \
  -q '(call_expression
        function: (field_expression
          field: (field_identifier) @m (#eq? @m "unwrap")))' \
  -p ./src
```

**All .expect() calls:**

```bash
doora \
  -q '(call_expression
        function: (field_expression
          field: (field_identifier) @m (#eq? @m "expect")))' \
  -p ./src
```

**All .clone() calls:**

```bash
doora \
  -q '(call_expression
        function: (field_expression
          field: (field_identifier) @m (#eq? @m "clone")))' \
  -p ./src
```

**All function calls by name:**

```bash
doora \
  -q '(call_expression
        function: (identifier) @fn (#eq? @fn "parse_file"))' \
  -p ./src
```

### TODO and FIXME comments

Tree-sitter's Rust grammar parses `//` line comments as `line_comment` nodes and `/* */` block comments as `block_comment` nodes. The full comment text (including the `//` prefix) is the node's text.

```bash
# All TODO comments
doora \
  -q '(line_comment) @c (#match? @c "TODO")' \
  -p ./src

# All FIXME comments
doora \
  -q '(line_comment) @c (#match? @c "FIXME")' \
  -p ./src

# All HACK and FIXME and TODO in one pass
doora \
  -q '(line_comment) @c (#match? @c "(TODO|FIXME|HACK|XXX)")' \
  -p ./src

# Block comments containing TODO
doora \
  -q '(block_comment) @c (#match? @c "TODO")' \
  -p ./src
```

### Use declarations (imports)

**All use declarations:**

```bash
doora -q '(use_declaration) @import' -p ./src
```

**Imports from a specific crate:**

```bash
doora \
  -q '(use_declaration (scoped_identifier path: (identifier) @crate (#eq? @crate "std")))' \
  -p ./src
```

### Let bindings

**All let bindings:**

```bash
doora -q '(let_declaration pattern: (identifier) @var_name)' -p ./src
```

**Let bindings with a specific type annotation:**

```bash
doora \
  -q '(let_declaration
        pattern: (identifier) @var_name
        type: (generic_type
          type: (type_identifier) @t (#eq? @t "Vec")))' \
  -p ./src
```

---

## Python

### Function definitions

**All function definitions:**

```bash
doora -q '(function_definition name: (identifier) @fn_name)' -p . --lang python
```

**Test functions:**

```bash
doora \
  -q '(function_definition name: (identifier) @fn (#match? @fn "^test_"))' \
  -p . --lang python
```

**Async functions:**

```bash
doora \
  -q '(function_definition
        "async"
        name: (identifier) @fn_name)' \
  -p . --lang python
```

**Functions with a specific argument count:**

```bash
# Functions with exactly one parameter
doora \
  -q '(function_definition
        name: (identifier) @fn_name
        parameters: (parameters
          . (identifier) . ")"))' \
  -p . --lang python

# Functions with self as the first parameter (methods)
doora \
  -q '(function_definition
        name: (identifier) @fn_name
        parameters: (parameters
          . (identifier) @first (#eq? @first "self")))' \
  -p . --lang python
```

**Functions with type annotations:**

```bash
doora \
  -q '(function_definition
        name: (identifier) @fn_name
        return_type: (_) @return_type)' \
  -p . --lang python
```

### Class definitions

**All class definitions:**

```bash
doora -q '(class_definition name: (identifier) @class_name)' -p . --lang python
```

**Classes inheriting from a specific base:**

```bash
doora \
  -q '(class_definition
        name: (identifier) @class_name
        superclasses: (argument_list
          (identifier) @base (#eq? @base "Exception")))' \
  -p . --lang python
```

**Methods inside classes:**

```bash
doora \
  -q '(class_definition
        body: (block
          (function_definition name: (identifier) @method_name)))' \
  -p . --lang python
```

**Class attributes (instance variables):**

```bash
doora \
  -q '(class_definition
        body: (block
          (expression_statement
            (assignment left: (identifier) @attr_name))))' \
  -p . --lang python
```

### Decorators

**All decorated definitions:**

```bash
doora -q '(decorated_definition decorator: (decorator) @decorator)' -p . --lang python
```

**Specific decorator:**

```bash
# Functions decorated with @property
doora \
  -q '(decorated_definition
        decorator: (decorator (identifier) @d (#eq? @d "property"))
        definition: (function_definition name: (identifier) @fn_name))' \
  -p . --lang python

# Flask routes (@app.route)
doora \
  -q '(decorated_definition
        decorator: (decorator) @dec (#match? @dec "route")
        definition: (function_definition name: (identifier) @fn_name))' \
  -p . --lang python
```

### Imports

**All import statements:**

```bash
doora -q '(import_statement) @import' -p . --lang python
doora -q '(import_from_statement) @from_import' -p . --lang python
```

**Import from a specific module:**

```bash
doora \
  -q '(import_from_statement
        module_name: (dotted_name) @mod (#eq? @mod "os"))' \
  -p . --lang python
```

### TODO comments

```bash
doora \
  -q '(comment) @c (#match? @c "(TODO|FIXME|HACK)")' \
  -p . --lang python
```

### String literals

**All string literals:**

```bash
doora -q '(string) @str' -p . --lang python
```

**Docstrings (first expression in a function body):**

```bash
doora \
  -q '(function_definition
        name: (identifier) @fn_name
        body: (block . (expression_statement (string) @docstring)))' \
  -p . --lang python
```

---

## JavaScript

### Function definitions

**Function declarations:**

```bash
doora -q '(function_declaration name: (identifier) @fn_name)' -p . --lang js
```

**Arrow functions assigned to variables:**

```bash
doora \
  -q '(lexical_declaration
        (variable_declarator
          name: (identifier) @fn_name
          value: (arrow_function)))' \
  -p . --lang js
```

**Arrow functions with specific parameter count:**

```bash
# Arrow functions with exactly one parameter
doora \
  -q '(arrow_function
        parameter: (identifier) @param_name)' \
  -p . --lang js
```

**Async functions:**

```bash
doora \
  -q '(function_declaration
        "async"
        name: (identifier) @fn_name)' \
  -p . --lang js
```

**Generator functions:**

```bash
doora \
  -q '(generator_function
        name: (identifier) @fn_name)' \
  -p . --lang js
```

### Class definitions

**All class declarations:**

```bash
doora -q '(class_declaration name: (identifier) @class_name)' -p . --lang js
```

**Classes extending a specific base:**

```bash
doora \
  -q '(class_declaration
        name: (identifier) @class_name
        (class_heritage
          (identifier) @base (#eq? @base "React.Component")))' \
  -p . --lang js
```

**Method definitions:**

```bash
# All methods
doora -q '(method_definition name: (property_identifier) @method_name)' -p . --lang js

# Async methods
doora \
  -q '(method_definition
        "async"
        name: (property_identifier) @method_name)' \
  -p . --lang js

# Static methods
doora \
  -q '(method_definition
        "static"
        name: (property_identifier) @method_name)' \
  -p . --lang js

# Getter methods
doora \
  -q '(method_definition
        "get"
        name: (property_identifier) @getter_name)' \
  -p . --lang js
```

### Imports and exports

**ES module imports:**

```bash
doora -q '(import_declaration) @import' -p . --lang js
```

**Import from a specific module:**

```bash
doora \
  -q '(import_declaration
        source: (string) @src (#match? @src "react"))' \
  -p . --lang js
```

**All exports:**

```bash
doora -q '(export_statement) @export' -p . --lang js
```

**Default exports:**

```bash
doora -q '(export_statement "default") @export' -p . --lang js
```

### Call expressions

**All function calls:**

```bash
doora \
  -q '(call_expression function: (identifier) @fn_name)' \
  -p . --lang js
```

**Console.log calls:**

```bash
doora \
  -q '(call_expression
        function: (member_expression
          object: (identifier) @obj (#eq? @obj "console")
          property: (property_identifier) @m (#eq? @m "log")))' \
  -p . --lang js
```

### TODO comments

```bash
doora \
  -q '(comment) @c (#match? @c "(TODO|FIXME|HACK)")' \
  -p . --lang js
```

---

## TypeScript

### Function definitions

**All function declarations:**

```bash
doora -q '(function_declaration name: (identifier) @fn_name)' -p . --lang ts
```

**Generic functions:**

```bash
doora \
  -q '(function_declaration
        name: (identifier) @fn_name
        type_parameters: (type_parameters))' \
  -p . --lang ts
```

**Async functions:**

```bash
doora \
  -q '(function_declaration
        "async"
        name: (identifier) @fn_name)' \
  -p . --lang ts
```

### Interface definitions

**All interfaces:**

```bash
doora -q '(interface_declaration name: (type_identifier) @interface_name)' -p . --lang ts
```

**A specific interface:**

```bash
doora \
  -q '(interface_declaration name: (type_identifier) @n (#eq? @n "Repository"))' \
  -p . --lang ts
```

**Interfaces extending another:**

```bash
doora \
  -q '(interface_declaration
        name: (type_identifier) @interface_name
        (extends_type_clause
          (type_identifier) @extends (#eq? @extends "BaseEntity")))' \
  -p . --lang ts
```

**Method signatures inside interfaces:**

```bash
doora \
  -q '(interface_declaration
        name: (type_identifier) @interface_name
        body: (object_type
          (method_signature
            name: (property_identifier) @method_name)))' \
  -p . --lang ts
```

### Type aliases

**All type aliases:**

```bash
doora -q '(type_alias_declaration name: (type_identifier) @type_name)' -p . --lang ts
```

**Union types:**

```bash
doora \
  -q '(type_alias_declaration
        name: (type_identifier) @type_name
        value: (union_type))' \
  -p . --lang ts
```

### Enum declarations

**All enums:**

```bash
doora -q '(enum_declaration name: (identifier) @enum_name)' -p . --lang ts
```

**Const enums:**

```bash
doora \
  -q '(enum_declaration
        "const"
        name: (identifier) @enum_name)' \
  -p . --lang ts
```

### Class definitions

**All classes:**

```bash
doora -q '(class_declaration name: (identifier) @class_name)' -p . --lang ts
```

**Abstract classes:**

```bash
doora \
  -q '(class_declaration
        "abstract"
        name: (identifier) @class_name)' \
  -p . --lang ts
```

**Classes implementing an interface:**

```bash
doora \
  -q '(class_declaration
        name: (identifier) @class_name
        (implements_clause
          (type_identifier) @interface (#eq? @interface "Repository")))' \
  -p . --lang ts
```

**Property declarations with specific types:**

```bash
# Properties typed as string
doora \
  -q '(public_field_definition
        name: (property_identifier) @prop_name
        type: (type_annotation (predefined_type) @t (#eq? @t "string")))' \
  -p . --lang ts

# Optional properties (prop?: Type)
doora \
  -q '(public_field_definition
        name: (property_identifier) @prop_name
        "?")' \
  -p . --lang ts
```

### Decorators (Angular, NestJS, etc.)

**All decorated classes:**

```bash
doora \
  -q '(decorator (identifier) @d)' \
  -p . --lang ts
```

**Specific decorator:**

```bash
doora \
  -q '(class_declaration
        (decorator (identifier) @d (#eq? @d "Injectable")))' \
  -p . --lang ts
```

### TODO comments

```bash
doora \
  -q '(comment) @c (#match? @c "(TODO|FIXME|HACK)")' \
  -p . --lang ts
```

---

## Go

### Function definitions

**All function declarations (not methods):**

```bash
doora -q '(function_declaration name: (identifier) @fn_name)' -p . --lang go
```

**A specific function:**

```bash
doora \
  -q '(function_declaration name: (identifier) @fn (#eq? @fn "main"))' \
  -p . --lang go
```

**Functions with a specific naming pattern:**

```bash
doora \
  -q '(function_declaration name: (identifier) @fn (#match? @fn "^New"))' \
  -p . --lang go
```

**Functions with specific parameter count:**

```bash
# Functions with exactly one parameter
doora \
  -q '(function_declaration
        name: (identifier) @fn_name
        parameters: (parameter_list
          . (parameter_declaration) . ")"))' \
  -p . --lang go

# Functions with no parameters
doora \
  -q '(function_declaration
        name: (identifier) @fn_name
        parameters: (parameter_list . ")"))' \
  -p . --lang go
```

**Functions returning error:**

```bash
doora \
  -q '(function_declaration
        name: (identifier) @fn_name
        result: (parameter_list
          (type_identifier) @t (#eq? @t "error")))' \
  -p . --lang go
```

**Functions returning multiple values:**

```bash
doora \
  -q '(function_declaration
        name: (identifier) @fn_name
        result: (parameter_list))' \
  -p . --lang go
```

### Method declarations

**All methods (with receivers):**

```bash
doora -q '(method_declaration name: (field_identifier) @method_name)' -p . --lang go
```

**Methods on a specific type:**

```bash
doora \
  -q '(method_declaration
        receiver: (parameter_list
          (parameter_declaration
            type: (type_identifier) @recv (#eq? @recv "Config")))
        name: (field_identifier) @method_name)' \
  -p . --lang go
```

**Pointer receiver methods:**

```bash
doora \
  -q '(method_declaration
        receiver: (parameter_list
          (parameter_declaration
            type: (pointer_type
              (type_identifier) @recv)))
        name: (field_identifier) @method_name)' \
  -p . --lang go
```

### Type declarations (structs and interfaces)

**All type declarations:**

```bash
doora \
  -q '(type_declaration (type_spec name: (type_identifier) @type_name))' \
  -p . --lang go
```

**Struct type declarations:**

```bash
doora \
  -q '(type_declaration
        (type_spec
          name: (type_identifier) @struct_name
          type: (struct_type)))' \
  -p . --lang go
```

**Interface type declarations:**

```bash
doora \
  -q '(type_declaration
        (type_spec
          name: (type_identifier) @interface_name
          type: (interface_type)))' \
  -p . --lang go
```

**Struct fields of a specific type:**

```bash
# Fields of type string
doora \
  -q '(field_declaration
        name: (field_identifier) @field_name
        type: (type_identifier) @t (#eq? @t "string"))' \
  -p . --lang go

# Fields of pointer type
doora \
  -q '(field_declaration
        name: (field_identifier) @field_name
        type: (pointer_type))' \
  -p . --lang go
```

### Imports

**All import declarations:**

```bash
doora -q '(import_declaration) @import' -p . --lang go
```

**Import of a specific package:**

```bash
doora \
  -q '(import_spec
        path: (interpreted_string_literal) @path (#match? @path "context"))' \
  -p . --lang go
```

### Error handling patterns

**All if err != nil blocks:**

```bash
doora \
  -q '(if_statement
        condition: (binary_expression
          left: (identifier) @err (#eq? @err "err")
          "!="
          right: (nil)))' \
  -p . --lang go
```

**All error return statements:**

```bash
doora \
  -q '(return_statement
        (expression_list
          (identifier) @err (#eq? @err "err")))' \
  -p . --lang go
```

### TODO comments

```bash
doora \
  -q '(comment) @c (#match? @c "(TODO|FIXME|HACK)")' \
  -p . --lang go
```

---

## C

### Function definitions

**All function definitions:**

```bash
doora \
  -q '(function_definition
        declarator: (function_declarator
          declarator: (identifier) @fn_name))' \
  -p . --lang c
```

**A specific function:**

```bash
doora \
  -q '(function_definition
        declarator: (function_declarator
          declarator: (identifier) @fn (#eq? @fn "main")))' \
  -p . --lang c
```

**Functions matching a pattern:**

```bash
doora \
  -q '(function_definition
        declarator: (function_declarator
          declarator: (identifier) @fn (#match? @fn "^handle_")))' \
  -p . --lang c
```

**Static functions:**

```bash
doora \
  -q '(function_definition
        (storage_class_specifier) @s (#eq? @s "static")
        declarator: (function_declarator
          declarator: (identifier) @fn_name))' \
  -p . --lang c
```

**Functions with a specific return type:**

```bash
# Functions returning int
doora \
  -q '(function_definition
        type: (primitive_type) @t (#eq? @t "int")
        declarator: (function_declarator
          declarator: (identifier) @fn_name))' \
  -p . --lang c

# Functions returning void
doora \
  -q '(function_definition
        type: (primitive_type) @t (#eq? @t "void")
        declarator: (function_declarator
          declarator: (identifier) @fn_name))' \
  -p . --lang c
```

**Functions with specific argument count:**

```bash
# Functions with exactly two parameters
doora \
  -q '(function_definition
        declarator: (function_declarator
          declarator: (identifier) @fn_name
          parameters: (parameter_list
            . (parameter_declaration) . (parameter_declaration) . ")")))' \
  -p . --lang c
```

### Struct definitions

**All struct specifiers with a name:**

```bash
doora -q '(struct_specifier name: (type_identifier) @struct_name)' -p . --lang c
```

**Typedef structs:**

```bash
doora \
  -q '(type_definition
        type: (struct_specifier)
        declarator: (type_identifier) @typedef_name)' \
  -p . --lang c
```

**Struct fields of a specific type:**

```bash
# char* fields (strings)
doora \
  -q '(field_declaration
        type: (type_identifier) @t (#eq? @t "char")
        declarator: (pointer_declarator
          declarator: (field_identifier) @field_name))' \
  -p . --lang c

# int fields
doora \
  -q '(field_declaration
        type: (primitive_type) @t (#eq? @t "int")
        declarator: (field_identifier) @field_name)' \
  -p . --lang c
```

### Enum definitions

**All enums:**

```bash
doora -q '(enum_specifier name: (type_identifier) @enum_name)' -p . --lang c
```

### Preprocessor directives

**All includes:**

```bash
doora -q '(preproc_include) @include' -p . --lang c
```

**Include of a specific header:**

```bash
doora \
  -q '(preproc_include path: (string_literal) @path (#match? @path "stdio"))' \
  -p . --lang c
```

**All macro definitions:**

```bash
doora -q '(preproc_def name: (identifier) @macro_name)' -p . --lang c
```

**Macro definitions matching a pattern:**

```bash
doora \
  -q '(preproc_def name: (identifier) @m (#match? @m "^MAX_"))' \
  -p . --lang c
```

**Function-like macro definitions:**

```bash
doora -q '(preproc_function_def name: (identifier) @macro_name)' -p . --lang c
```

### TODO comments

```bash
doora \
  -q '(comment) @c (#match? @c "(TODO|FIXME|HACK)")' \
  -p . --lang c
```

---

## C++

### Function definitions

**Free function definitions:**

```bash
doora \
  -q '(function_definition
        declarator: (function_declarator
          declarator: (identifier) @fn_name))' \
  -p . --lang cpp
```

**Virtual functions:**

```bash
doora \
  -q '(virtual_function_definition
        declarator: (function_declarator
          declarator: (field_identifier) @fn_name))' \
  -p . --lang cpp
```

**Pure virtual functions (= 0):**

```bash
doora \
  -q '(field_declaration
        (virtual)
        declarator: (function_declarator
          declarator: (field_identifier) @fn_name)
        (pure_virtual_clause))' \
  -p . --lang cpp
```

**Override methods:**

```bash
doora \
  -q '(function_definition
        declarator: (function_declarator
          declarator: (field_identifier) @fn_name
          (type_qualifier) @q (#eq? @q "override")))' \
  -p . --lang cpp
```

**Template functions:**

```bash
doora \
  -q '(template_declaration
        (function_definition
          declarator: (function_declarator
            declarator: (identifier) @fn_name)))' \
  -p . --lang cpp
```

### Class definitions

**All class declarations:**

```bash
doora -q '(class_specifier name: (type_identifier) @class_name)' -p . --lang cpp
```

**Classes inheriting from a specific base:**

```bash
doora \
  -q '(class_specifier
        name: (type_identifier) @class_name
        (base_class_clause
          (type_identifier) @base (#eq? @base "Animal")))' \
  -p . --lang cpp
```

**Abstract classes (contain pure virtual functions):**

```bash
doora \
  -q '(class_specifier
        name: (type_identifier) @class_name
        body: (field_declaration_list
          (field_declaration
            (pure_virtual_clause))))' \
  -p . --lang cpp
```

### Struct definitions

**All structs:**

```bash
doora -q '(struct_specifier name: (type_identifier) @struct_name)' -p . --lang cpp
```

**Struct fields of a specific type:**

```bash
# std::string fields
doora \
  -q '(field_declaration
        type: (qualified_identifier
          scope: (namespace_identifier) @ns (#eq? @ns "std")
          name: (type_identifier) @t (#eq? @t "string"))
        declarator: (field_identifier) @field_name)' \
  -p . --lang cpp
```

### Namespaces

**All namespace declarations:**

```bash
doora -q '(namespace_definition name: (namespace_identifier) @ns_name)' -p . --lang cpp
```

**A specific namespace:**

```bash
doora \
  -q '(namespace_definition name: (namespace_identifier) @ns (#eq? @ns "detail"))' \
  -p . --lang cpp
```

### Templates

**All template declarations:**

```bash
doora -q '(template_declaration) @template' -p . --lang cpp
```

**Template class specializations:**

```bash
doora \
  -q '(template_declaration
        (class_specifier name: (type_identifier) @class_name))' \
  -p . --lang cpp
```

### Include directives

**All includes:**

```bash
doora -q '(preproc_include) @include' -p . --lang cpp
```

**System headers:**

```bash
doora \
  -q '(preproc_include path: (system_lib_string) @header)' \
  -p . --lang cpp
```

### TODO comments

```bash
doora \
  -q '(comment) @c (#match? @c "(TODO|FIXME|HACK)")' \
  -p . --lang cpp
```

---

## Cross-Language Patterns

These patterns work across multiple languages simultaneously using `--lang auto` (the default).

### Find all function definitions in a mixed repo

`function_item` compiles only for Rust. `function_definition` compiles only for Python. `function_declaration` compiles for JS, TS, Go, C, and C++. In auto mode, each query is compiled against every grammar and languages where it fails are skipped:

```bash
# Search only Rust files (function_item is Rust-specific)
doora -q '(function_item name: (identifier) @fn_name)' -p .

# Search JS, TS, Go, C, C++ files (function_declaration exists in all of them)
doora -q '(function_declaration name: (identifier) @fn_name)' -p .

# Search only Python files (function_definition is Python-specific)
doora -q '(function_definition name: (identifier) @fn_name)' -p .
```

### Find all TODO comments across all languages

```bash
doora -q '(comment) @c (#match? @c "TODO")' -p .
```

Note: Rust uses `line_comment` and `block_comment` node types. If targeting Rust specifically, use those instead:

```bash
# For Rust (includes both // and /* */ comments in one query)
doora -q '(line_comment) @c (#match? @c "TODO")' -p . --lang rust
doora -q '(block_comment) @c (#match? @c "TODO")' -p . --lang rust
```

### Find all identifier nodes containing a specific word

```bash
# Every identifier that contains "auth" anywhere in the name
doora -q '(identifier) @id (#match? @id "auth")' -p .
```

---

## Predicate Reference

All predicates appear inside an S-expression after the structural pattern they filter.

### `#eq?` — exact string equality

```scheme
(function_item name: (identifier) @fn (#eq? @fn "connect"))
```

Case-sensitive. The entire captured text must equal the string exactly.

### `#match?` — regular expression

```scheme
(function_item name: (identifier) @fn (#match? @fn "^handle_"))
```

The regex is compiled once at query compile time (not per file). Uses Rust's `regex` crate syntax. Anchors work as expected: `^` = start of captured text, `$` = end.

**Common regex patterns:**

| Pattern | Matches |
|---|---|
| `^handle_` | Names starting with `handle_` |
| `_error$` | Names ending with `_error` |
| `^(get\|set\|del)_` | Names starting with `get_`, `set_`, or `del_` |
| `[Tt]est` | Names containing `Test` or `test` |
| `\d+` | Names containing digits |
| `^[A-Z]` | Names starting with uppercase (PascalCase) |
| `^[a-z]` | Names starting with lowercase |

### `#not-eq?` — negative exact equality

```scheme
(function_item name: (identifier) @fn (#not-eq? @fn "main"))
```

Excludes matches where the captured text equals the string.

### `#any-of?` — match any value in a list

```scheme
(function_item
  name: (identifier) @fn
  (#any-of? @fn "get" "set" "delete" "create" "update"))
```

More readable than a long `#match?` alternation for short exact lists.

---

## Advanced Patterns

### Anchored children (`.` operator)

The `.` operator anchors position within a list. `(node . child)` means `child` is the first child. `(node child .)` means `child` is the last child.

```scheme
; The first statement in a block
(block . (expression_statement) @first)

; The last statement in a block
(block (expression_statement) @last .)

; A function with exactly two parameters
(function_item
  parameters: (parameters . (parameter) . (parameter) . ")"))
```

### Nested capture — capture the whole parent and a child

```scheme
; Capture both the function item AND its name
(function_item @whole_fn
  name: (identifier) @fn_name)
```

Both `@whole_fn` and `@fn_name` appear in output. Each produces a separate result line.

### Multiple predicates (AND logic)

Multiple predicates on the same query all must be satisfied:

```scheme
; Async function AND name starts with handle_
(function_item
  (function_modifiers "async")
  name: (identifier) @fn
  (#match? @fn "^handle_"))
```

### OR logic via multiple `-q` flags

Multiple queries are OR'd — a file is searched with each query independently:

```bash
# Finds functions named "foo" OR structs named "Bar"
doora \
  -q '(function_item name: (identifier) @fn (#eq? @fn "foo"))' \
  -q '(struct_item name: (type_identifier) @s (#eq? @s "Bar"))' \
  -p ./src
```

### Wildcard node type `(_)`

Matches any single node regardless of type:

```scheme
; Any node named "connect" (works across different node types)
((_) @node (#eq? @node "connect"))
```

### Finding code patterns across call chains

```scheme
; obj.method().unwrap() — unwrap after a method call
(call_expression
  function: (field_expression
    value: (call_expression)
    field: (field_identifier) @m (#eq? @m "unwrap")))
```

### Finding string literals with specific content

```scheme
; Rust string literal containing a specific URL
(string_literal) @s (#match? @s "https://")

; Python f-string
(interpolated_string) @fstr
```

---

## Troubleshooting

### "Query did not compile against any supported language"

The node type in your query does not exist in any supported grammar. Common causes:

- Used `function_item` in auto mode — it only exists in Rust. Add `--lang rust`.
- Typo in node type name — node types are case-sensitive and use underscores.
- Using the wrong language flag — `function_definition` is Python, not Rust.

**Debug:** Use the TUI's AST pane (`--tui`) to see the exact node kinds for your code.

### Query matches too many results (false positives)

Add predicates to narrow the match:

```bash
# Too broad — matches all identifiers named "connect"
doora -q '(identifier) @id (#eq? @id "connect")' -p ./src

# Narrowed — only function definitions named "connect"
doora \
  -q '(function_item name: (identifier) @fn (#eq? @fn "connect"))' \
  -p ./src
```

### Query matches nothing (false negatives)

1. Verify the node type: open the TUI and inspect the CST of a file that should match.
2. Check field names: `name:` is a named field in `function_item` but may differ by language.
3. Remove predicates one by one to see which constraint is over-filtering.
4. Use `(source_file) @root` to verify the parser is processing your files at all.

### Slow queries on large repositories

1. Add string literal predicates (`#eq?` or `#match?`) so the Bloom filter index can reject non-matching files before parsing.
2. Build the index first: `doora index ./src`
3. Use `--stats` to see how many files the sieve is rejecting.

### Column numbers seem wrong

All column numbers are **byte offsets**, not character counts. For ASCII-only source this is identical to character position. For multi-byte UTF-8 source (emoji, accented characters, CJK), the column offset reflects bytes, matching how Tree-sitter and most editors with LSP integration count positions.

---

## Quick Reference Card

```
Find function definitions
  Rust:       (function_item name: (identifier) @fn_name)
  Python:     (function_definition name: (identifier) @fn_name)
  JS/TS/Go:   (function_declaration name: (identifier) @fn_name)
  C/C++:      (function_definition declarator: (function_declarator declarator: (identifier) @fn_name))

Find class/struct definitions
  Rust:       (struct_item name: (type_identifier) @name)
  Python:     (class_definition name: (identifier) @name)
  JS/TS:      (class_declaration name: (identifier) @name)
  Go:         (type_declaration (type_spec name: (type_identifier) @name type: (struct_type)))
  C/C++:      (struct_specifier name: (type_identifier) @name)
  C++:        (class_specifier name: (type_identifier) @name)

Find TODO comments (all languages)
              (comment) @c (#match? @c "TODO")
  Rust only:  (line_comment) @c (#match? @c "TODO")

Find unwrap() calls (Rust)
              (call_expression function: (field_expression field: (field_identifier) @m (#eq? @m "unwrap")))

Filter by name (add to any query)
  Exact:      (#eq? @capture "name")
  Regex:      (#match? @capture "^pattern")
  Exclude:    (#not-eq? @capture "excluded")
  List:       (#any-of? @capture "a" "b" "c")
```