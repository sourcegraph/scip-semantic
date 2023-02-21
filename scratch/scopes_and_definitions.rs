#[derive(Debug)]
struct ScopeNode<'a> {
    name: String,
    node: Node<'a>,
}

#[derive(Debug)]
struct LocalScope<'a> {
    root: ScopeNode<'a>,
    children: Vec<LocalScope<'a>>,
}

impl<'a> LocalScope<'a> {
    fn add_child_node(&mut self, root: ScopeNode<'a>) {
        // We could probably be kind of cute with this here and insert these in a sorted fashio
        // so that we can binary search through the scopes.
        //
        // TODO: Push the nested scope values?
        self.children.push(LocalScope {
            root: ScopeNode {
                // Update scope names as we append
                name: self.root.name.clone() + "." + &root.name,
                ..root
            },
            children: Vec::new(),
        });
    }

    fn containing_scope(&'a self, node: Node<'a>) -> Option<&'a ScopeNode<'a>> {
        for child in &self.children {
            let containing = child.containing_scope(node);
            if containing.is_some() {
                return containing;
            }
        }

        // dbg!((
        //     self.root.node.start_position(),
        //     self.root.node.end_position()
        // ));
        //
        // dbg!((node.start_position(), node.end_position()));
        //
        // dbg!(node.descendant_for_point_range(
        //     self.root.node.start_position(),
        //     self.root.node.end_position()
        // ));

        node_contains(&self.root.node, &node).then_some(&self.root)

        // if self
        //     .root
        //     .node
        //     .descendant_for_point_range(node.start_position(), node.end_position())
        //     .is_some()
        // {
        //     return Some(&self.root);
        // }

        // None
    }
}

#[derive(Debug)]
struct RawMatchedQuery<'a> {
    kind: String,
    node: Node<'a>,
    name: Node<'a>,
    _parent: Option<Node<'a>>,
}

#[derive(Debug)]
struct FileMatcher<'a> {
    _source: &'a str,
    bytes: &'a [u8],
    lines: Vec<&'a str>,
    matched: Vec<RawMatchedQuery<'a>>,
    resolved: Vec<Option<TagEntry>>,
}

impl<'a> FileMatcher<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            _source: source,
            bytes: source.as_bytes(),
            lines: source.lines().collect(),
            matched: Vec::new(),
            resolved: Vec::new(),
        }
    }

    fn add_match(
        &mut self,
        kind: String,
        node: Node<'a>,
        name: Node<'a>,
        parent: Option<Node<'a>>,
    ) {
        self.matched.push(RawMatchedQuery {
            kind,
            node,
            name,
            _parent: parent,
        });
        self.resolved.push(None);
    }

    fn resolve_match(&mut self, scope: &LocalScope, idx: usize) {
        // Only resolve once
        if self.resolved[idx].is_some() {
            return;
        }

        let matched = &self.matched[idx];
        let node = matched.node;

        if false {
            dbg!(self.lines[node.start_position().row]);
        }

        let text = matched.name.utf8_text(self.bytes).unwrap();
        let scoped = dbg!(scope.containing_scope(node)).unwrap();

        let entry = TagEntry {
            symbol: scoped.name.clone() + "." + text,
            kind: matched.kind.parse().unwrap(),
            line: node.start_position().row,

            parent: None,
        };

        self.resolved[idx] = Some(entry);
    }
}

fn show_go() {
    let mut parser = Parser::new();

    let lang = "rust";
    let (language, source_code, scopes_query, definition_query) = match lang {
        "go" => {
            let language = tree_sitter_go::language();
            let source_code = include_str!("../../testdata/example.go");
            let query_str = "
                (function_declaration name: (identifier) @definition.function)
                (method_declaration

                    receiver:
                        (parameter_list
                            (parameter_declaration
                                type: (pointer_type (type_identifier) @parent)))
                    name: (field_identifier) @definition.method)

                (method_declaration
                    receiver:
                        (parameter_list
                            (parameter_declaration
                                type: (type_identifier) @parent))
                    name: (field_identifier) @definition.method)

                (type_declaration (type_spec name: (type_identifier) @definition.type))
            ";
            let definition_query = Query::new(language, query_str).unwrap();

            let scopes_query = Query::new(language, "").unwrap();

            (language, source_code, scopes_query, definition_query)
        }
        "rust" => {
            let language = tree_sitter_rust::language();
            let source_code = include_str!("../../testdata/scopes.rs");
            let scopes_query = "
                    ;; [
                    ;;  (function_item)
                    ;;  (while_expression)
                    ;;  (struct_item)
                    ;;  (enum_item)
                    ;; ] @scope.global

                    (trait_item
                     name: (_) @name.type) @scope.global

                    (impl_item
                      trait: (_) @name.type
                      type: (_) @name.type) @scope.global

                    ;; [
                    ;;  (block)
                    ;;  (closure_expression)
                    ;;  (for_expression)
                    ;;  (loop_expression)
                    ;;  (if_expression)
                    ;;  (match_expression)
                    ;;  (match_arm)
                    ;; ] @scope.local
                ";

            let defintion_query = "
                (function_signature_item
                 name: (identifier) @name) @definition.function

                (function_item
                 name: (identifier) @name) @definition.function

                (struct_item
                 name: (type_identifier) @name) @definition.type
                ";

            (
                language,
                source_code,
                Query::new(language, scopes_query).unwrap(),
                Query::new(language, defintion_query).unwrap(),
            )
        }
        _ => unreachable!("oh no no"),
    };

    let source_bytes = source_code.as_bytes();

    parser.set_language(language).unwrap();

    let tree = parser.parse(source_code, None).unwrap();
    let root_node = tree.root_node();
    if false {
        walk_child(&root_node, 0);
    }

    let mut cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(&scopes_query, root_node, source_bytes);

    // TODO: This could be the default or we have to always say something like `source_file` as a
    // match? Is that standard for all tree-sitter grammars or not?
    let mut scopes = vec![];

    for m in matches {
        let mut node = None;

        let mut names = vec![];
        for c in m.captures {
            // TODO: This should account for the type of name it is.
            if scopes_query
                //  We can use scip descriptors later to do this correctly as well.
                .capture_names()
                .get(c.index as usize)
                .unwrap()
                .starts_with("name")
            {
                names.push(c.node.utf8_text(source_bytes).unwrap());
            }

            if scopes_query
                .capture_names()
                .get(c.index as usize)
                .unwrap()
                .starts_with("scope")
            {
                node = Some(c.node);
            }
        }

        let name = names.join(".");
        scopes.push(ScopeNode {
            name,
            node: node.unwrap(),
        });
    }

    let mut scope = LocalScope {
        root: ScopeNode {
            name: "package".to_string(),
            node: root_node,
        },
        children: vec![],
    };

    scopes.sort_by(|a, b| {
        (a.node.end_byte() - a.node.start_byte()).cmp(&(b.node.end_byte() - b.node.start_byte()))
    });

    // TODO: Handle nested scopes
    while let Some(scope_node) = scopes.pop() {
        scope.add_child_node(scope_node);
    }

    dbg!(&scope);

    let mut cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(&definition_query, root_node, source_code.as_bytes());

    // Emit something similar to this
    // Another	testdata/example.go	/^func Another() {}$/;"	f	package:example
    // Something	testdata/example.go	/^func Something() {$/;"	f	package:example
    let mut file_matcher = FileMatcher::new(source_code);

    for m in matches {
        let mut kind = None;
        let mut node = None;
        let mut parent = None;
        let mut name = None;
        for capture in m.captures {
            let capture_kind = &definition_query.capture_names()[capture.index as usize];
            if capture_kind.starts_with("definition") {
                assert!(node.is_none());
                kind = Some(capture_kind.clone());
                node = Some(capture.node);
            }

            if capture_kind.starts_with("parent") {
                assert!(parent.is_none());
                parent = Some(capture.node);
            }

            if capture_kind.starts_with("name") {
                assert!(name.is_none());
                name = Some(capture.node);
            }
        }

        // let text = capture.node.utf8_text(source_code.as_bytes()).unwrap();
        // let line = source_lines[capture.node.start_position().row].replace('/', "\\/");
        file_matcher.add_match(kind.unwrap(), node.unwrap(), name.unwrap(), parent);
    }

    for idx in 0..file_matcher.matched.len() {
        file_matcher.resolve_match(&scope, idx);
    }

    dbg!(&file_matcher.resolved);
}
