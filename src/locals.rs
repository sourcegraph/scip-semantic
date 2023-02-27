use anyhow::Result;
use protobuf::Enum;
use scip::{
    symbol::format_symbol,
    types::{Occurrence, Symbol},
};
use scip_treesitter::prelude::*;
use tree_sitter::Node;

use crate::languages::LocalConfiguration;

#[derive(Debug)]
pub struct Scope<'a> {
    pub scope: Node<'a>,
    pub definitions: Vec<Definition<'a>>,
    pub references: Vec<Reference<'a>>,
    pub children: Vec<Scope<'a>>,
}

impl<'a> Scope<'a> {
    pub fn new(scope: Node<'a>) -> Self {
        Self {
            scope,
            definitions: vec![],
            references: vec![],
            children: vec![],
        }
    }

    // fn get_containing_node(&self, node: &Node<'a>) -> &Scope {
    //     if let Some(child) = self
    //         .children
    //         .iter()
    //         .find(|child| child.scope.contains_node(node))
    //     {
    //         child.get_containing_node(node)
    //     } else {
    //         self
    //     }
    // }

    pub fn insert_scope(&mut self, scope: Scope<'a>) {
        if let Some(child) = self
            .children
            .iter_mut()
            .find(|child| child.scope.contains_node(&scope.scope))
        {
            child.insert_scope(scope);
        } else {
            self.children.push(scope);
        }
    }

    pub fn insert_definition(&mut self, definition: Definition<'a>) {
        if let Some(child) = self
            .children
            .iter_mut()
            .find(|child| child.scope.contains_node(&definition.node))
        {
            child.insert_definition(definition)
        } else {
            self.definitions.push(definition);
        }
    }

    pub fn insert_reference(&mut self, reference: Reference<'a>) {
        if self.definitions.iter().any(|d| d.node == reference.node) {
            return;
        }

        if let Some(child) = self
            .children
            .iter_mut()
            .find(|child| child.scope.contains_node(&reference.node))
        {
            if child.definitions.iter().any(|d| d.node == reference.node) {
                return;
            }

            child.insert_reference(reference)
        } else {
            self.references.push(reference);
        }
    }

    fn stable_sort_definitions(&mut self) {
        self.definitions.sort_by_key(|item| item.node.start_byte());
        self.references.sort_by_key(|item| item.node.start_byte());

        self.children.sort_by_key(|item| item.scope.start_byte());
        self.children.iter_mut().for_each(|child| {
            child.stable_sort_definitions();
        });
    }

    pub fn into_occurrences(&mut self) -> Vec<Occurrence> {
        self.stable_sort_definitions();
        self.rec_into_occurrences(&mut 0)
    }

    fn occurrences_for_children(
        self: &Scope<'a>,
        def: &Definition<'a>,
        symbol: &str,
    ) -> Vec<Occurrence> {
        if self
            .definitions
            .iter()
            .any(|d| d.identifier == def.identifier)
        {
            // TODO: Should i return an option?
            return vec![];
        }

        // child.references
        let mut occurrences = vec![];
        for reference in &self.references {
            if reference.identifier == def.identifier {
                occurrences.push(scip::types::Occurrence {
                    range: reference.node.to_scip_range(),
                    symbol: symbol.to_string(),
                    ..Default::default()
                });
            }
        }

        occurrences.extend(
            self.children
                .iter()
                .flat_map(|c| c.occurrences_for_children(def, symbol)),
        );

        occurrences
    }

    fn rec_into_occurrences(&self, id: &mut usize) -> Vec<Occurrence> {
        // TODO: Need to think about how to make sure that we have stable sorting
        //  It may not be worth it for performance generally speaking,
        //  but probably worth it for our snapshot files

        let mut occurrences = vec![];
        for definition in &self.definitions {
            *id += 1;

            let symbol = format_symbol(Symbol::new_local(*id));
            let symbol_roles = scip::types::SymbolRole::Definition.value();

            occurrences.push(scip::types::Occurrence {
                range: definition.node.to_scip_range(),
                symbol: symbol.clone(),
                symbol_roles,
                // TODO:
                // syntax_kind: todo!(),
                ..Default::default()
            });

            for reference in &self.references {
                if reference.identifier == definition.identifier {
                    occurrences.push(scip::types::Occurrence {
                        range: reference.node.to_scip_range(),
                        symbol: symbol.clone(),
                        ..Default::default()
                    });
                }
            }

            occurrences.extend(
                self.children
                    .iter()
                    .flat_map(|c| c.occurrences_for_children(definition, symbol.as_str())),
            )
        }

        occurrences.extend(
            self.children
                .iter()
                .flat_map(|c| c.rec_into_occurrences(id)),
        );

        occurrences
    }
}

#[derive(Debug)]
pub struct Definition<'a> {
    pub group: &'a str,
    pub identifier: &'a str,
    pub node: Node<'a>,
}

#[derive(Debug)]
pub struct Reference<'a> {
    pub group: &'a str,
    pub identifier: &'a str,
    pub node: Node<'a>,
}

pub fn parse_tree<'a>(
    config: &mut LocalConfiguration,
    tree: &'a tree_sitter::Tree,
    source_bytes: &'a [u8],
) -> Result<Vec<scip::types::Occurrence>> {
    let mut cursor = tree_sitter::QueryCursor::new();

    let root_node = tree.root_node();
    let capture_names = config.query.capture_names();

    let mut scopes = vec![];
    let mut definitions = vec![];
    let mut references = vec![];

    for m in cursor.matches(&config.query, root_node, source_bytes) {
        let mut node = None;

        let mut scope = None;
        let mut definition = None;
        let mut reference = None;

        for capture in m.captures {
            let capture_name = capture_names
                .get(capture.index as usize)
                .expect("capture indexes should always work");

            node = Some(capture.node);

            if capture_name.starts_with("definition") {
                assert!(definition.is_none(), "only one definition per match");
                definition = Some(capture_name);
            }

            if capture_name.starts_with("reference") {
                assert!(reference.is_none(), "only one reference per match");
                reference = Some(capture_name);
            }

            if capture_name.starts_with("scope") {
                assert!(scope.is_none(), "declare only one scope per match");
                scope = Some(capture);
            }
        }

        let node = node.expect("there must always be at least one descriptor");

        if let Some(group) = definition {
            let identifier = node.utf8_text(source_bytes).expect("utf8_text");
            definitions.push(Definition {
                group,
                identifier,
                node,
            });
        } else if let Some(group) = reference {
            let identifier = node.utf8_text(source_bytes).expect("utf8_text");
            references.push(Reference {
                group,
                identifier,
                node,
            });
        } else {
            let scope =
                scope.expect("if there is no definition or reference, there must be a scope");
            scopes.push(Scope::new(scope.node));
        }
    }

    let mut root = Scope::new(root_node);

    // Sort largest to smallest scope to make sure that we get them in the right order
    scopes.sort_by_key(|m| {
        let node = m.scope;
        node.end_byte() - node.start_byte()
    });

    // Add all the scopes to our tree
    while let Some(m) = scopes.pop() {
        root.insert_scope(m);
    }

    while let Some(m) = definitions.pop() {
        root.insert_definition(m);
    }

    while let Some(m) = references.pop() {
        root.insert_reference(m);
    }

    let occs = root.into_occurrences();

    Ok(occs)
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use scip::types::Document;

    use super::*;
    use crate::{languages::LocalConfiguration, snapshot::dump_document};

    #[allow(dead_code)]
    fn parse_file_for_lang(config: &mut LocalConfiguration, source_code: &str) -> Result<Document> {
        let source_bytes = source_code.as_bytes();
        let tree = config.parser.parse(source_bytes, None).unwrap();

        let occ = parse_tree(config, &tree, source_bytes)?;
        let mut doc = Document::new();
        doc.occurrences = occ;
        doc.symbols = doc
            .occurrences
            .iter()
            .map(|o| scip::types::SymbolInformation {
                symbol: o.symbol.clone(),
                ..Default::default()
            })
            .collect();

        Ok(doc)
    }

    #[test]
    fn test_can_do_go() -> Result<()> {
        let mut config = crate::languages::go_locals();
        let source_code = include_str!("../testdata/locals.go");
        let doc = parse_file_for_lang(&mut config, source_code)?;

        let dumped = dump_document(&doc, source_code);
        insta::assert_snapshot!(dumped);

        Ok(())
    }

    #[test]
    fn test_can_do_nested_locals() -> Result<()> {
        let mut config = crate::languages::go_locals();
        let source_code = include_str!("../testdata/locals-nested.go");
        let doc = parse_file_for_lang(&mut config, source_code)?;

        let dumped = dump_document(&doc, source_code);
        insta::assert_snapshot!(dumped);

        Ok(())
    }
}
