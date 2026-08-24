(comment) @annotation

(type_declaration
    "type" @context
    [
        (type_spec
            name: (_) @name
            type_parameters: (_)? @context
            type: [
                (struct_type
                    "struct" @context)
                (interface_type
                    "interface" @context)
                [(array_type) (channel_type) (function_type) (generic_type) (map_type) (negated_type) (pointer_type) (qualified_type) (slice_type) (type_identifier) (parenthesized_type)] @context
            ]) @item
        (type_alias
            name: (_) @name
            type_parameters: (_)? @context
            "=" @context
            type: [
                (struct_type
                    "struct" @context)
                (interface_type
                    "interface" @context)
                [(array_type) (channel_type) (function_type) (generic_type) (map_type) (negated_type) (pointer_type) (qualified_type) (slice_type) (type_identifier) (parenthesized_type)] @context
            ]) @item
    ]
)

(function_declaration
    "func" @context
    name: (identifier) @name
    type_parameters: (_)? @context
    parameters: (parameter_list) @context
    result: (_)? @context) @item


(method_declaration
    "func" @context
    receiver: (parameter_list
        "(" @context
        (parameter_declaration
            name: (_) @context
            type: (_) @context)
        ")" @context)
    name: (field_identifier) @name
    parameters: (parameter_list) @context
    result: (_)? @context) @item

(const_declaration
    "const" @context
    (const_spec
        name: (identifier) @name
        type: (_)? @context
        "="? @context
        value: (_)? @context) @item)

(source_file
    (var_declaration
        "var" @context
        [
            ; The declaration may define multiple variables, and so @item is on
            ; the identifier so they get distinct ranges.
            (var_spec
                name: (identifier) @name @item
                type: (_) @context)
            (var_spec_list
                (var_spec
                    name: (identifier) @name @item
                    type: (_) @context)
            )
        ]
     )
)

(method_elem
    name: (_) @name
    parameters: (parameter_list) @context
    result: (_)? @context) @item

(interface_type
    (type_elem) @name @item)

; Fields declarations may define multiple fields, and so @item is on the
; declarator so they each get distinct ranges.
(field_declaration
    name: (_) @name @item
    type: (_) @context)
