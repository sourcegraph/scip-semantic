use tree_sitter::{Language, Parser, Query};

pub struct TagConfiguration {
    pub language: Language,
    pub query: Query,
    pub parser: Parser,
}

pub fn rust() -> TagConfiguration {
    let language = scip_treesitter_languages::rust();
    let query = "
        ;; TODO: Could do @scope.ignore to ignore this as a definition

        (mod_item
         name: (_) @descriptor.namespace) @scope

        (trait_item
         name: (_) @descriptor.type) @scope

        (impl_item
         trait: (_) @descriptor.type
         type: (_) @descriptor.type) @scope

        ;; TODO: @local to stop traversal
        (function_signature_item
         name: (identifier) @descriptor.method)

        ;; TODO: @local to stop traversal
        (function_item
         name: (identifier) @descriptor.method)

        (struct_item
         name: (type_identifier) @descriptor.type) @scope
        ";

    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    TagConfiguration {
        language,
        parser,
        query: Query::new(language, query).unwrap(),
    }
}

pub fn go() -> TagConfiguration {
    let language = scip_treesitter_languages::go();
    let query = "
        (source_file (package_clause (package_identifier) @descriptor.namespace)) @scope

        (function_declaration
         name: (identifier) @descriptor.method)

        (method_declaration
            receiver:
                (parameter_list
                    (parameter_declaration
                        type: (pointer_type (type_identifier) @descriptor.type)))
            name: (field_identifier) @descriptor.method)

        (method_declaration
            receiver:
                (parameter_list
                    (parameter_declaration
                        type: (type_identifier) @descriptor.type))
            name: (field_identifier) @descriptor.method)

        (type_declaration (type_spec name: (type_identifier) @descriptor.type))
        ";

    let mut parser = Parser::new();
    parser.set_language(language).unwrap();

    TagConfiguration {
        language,
        parser,
        query: Query::new(language, query).unwrap(),
    }
}
