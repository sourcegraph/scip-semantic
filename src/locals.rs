use std::collections::HashMap;

use anyhow::Result;
use protobuf::Enum;
use scip::{
    symbol::format_symbol,
    types::{Occurrence, Symbol},
};
use scip_treesitter::prelude::*;
use tree_sitter::Node;

use crate::languages::LocalConfiguration;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRange {
    start: usize,
    end: usize,
}

impl ByteRange {
    #[inline]
    pub fn contains(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.end
    }
}

#[derive(Debug)]
pub struct Scope<'a> {
    pub scope: Node<'a>,
    pub range: ByteRange,
    pub definitions: HashMap<&'a str, Definition<'a>>,
    // pub references: Vec<Reference<'a>>,
    pub references: HashMap<&'a str, Vec<Reference<'a>>>,
    pub children: Vec<Scope<'a>>,
}

impl<'a> Eq for Scope<'a> {}

impl<'a> PartialEq for Scope<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.scope.id() == other.scope.id()
    }
}

impl<'a> PartialOrd for Scope<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.range.partial_cmp(&other.range)
    }
}

impl<'a> Ord for Scope<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.range.cmp(&other.range)
    }
}

impl<'a> Scope<'a> {
    pub fn new(scope: Node<'a>) -> Self {
        Self {
            scope,
            range: ByteRange {
                start: scope.start_byte(),
                end: scope.end_byte(),
            },
            definitions: HashMap::new(),
            references: HashMap::new(),
            children: vec![],
        }
    }

    fn find_scopes_with(
        &'a self,
        scopes: &mut Vec<&Scope<'a>>,
        // predicate: impl Fn(&Scope<'a>) -> bool,
    ) {
        if self.definitions.is_empty() {
            scopes.push(self);
        }

        for child in &self.children {
            child.find_scopes_with(scopes);
        }
    }

    pub fn insert_scope(&mut self, scope: Scope<'a>) {
        if let Some(child) = self
            .children
            .iter_mut()
            .find(|child| self.range.contains(&child.range))
        {
            child.insert_scope(scope);
        } else {
            // TODO: We can insert this in a sorted fashion, then use binary search for the rest of
            // these from now on.
            //
            // Could consider not using a vec directly as well I suppose
            self.children.push(scope);
            // self.children.into_raw_parts
        }
    }

    pub fn insert_definition(&mut self, definition: Definition<'a>) {
        // TODO: Probably should assert that this the root node?
        if definition.scope_modifier == ScopeModifier::Global {
            self.definitions.insert(definition.identifier, definition);
            return;
        }

        if let Some(child) = self
            .children
            .iter_mut()
            .find(|child| child.range.contains(&definition.range))
        {
            child.insert_definition(definition)
        } else {
            self.definitions.insert(definition.identifier, definition);
        }
    }

    pub fn insert_reference(&mut self, reference: Reference<'a>) {
        if let Some(definition) = self.definitions.get(&reference.identifier) {
            if definition.node.id() == reference.node.id() {
                return;
            }
        }

        if let Some(child) = self
            .children
            .iter_mut()
            .find(|child| child.range.contains(&reference.range))
        {
            child.insert_reference(reference)
        } else {
            self.references
                .entry(reference.identifier)
                .or_default()
                .push(reference);
        }
    }

    fn stable_sort_definitions(&mut self) {
        // self.definitions.sort_by_key(|item| item.start_byte);
        // self.references.sort_by_key(|item| item.range.start);
        //
        // self.children.sort_by_key(|item| item.range.start);
        // self.children.iter_mut().for_each(|child| {
        //     child.stable_sort_definitions();
        // });
    }

    pub fn into_occurrences(&mut self, hint: usize) -> Vec<Occurrence> {
        self.stable_sort_definitions();

        let mut occs = Vec::with_capacity(hint);
        // TODO: This may be something useful to get more correct.
        occs.reserve(self.definitions.len() + self.references.len());
        self.rec_into_occurrences(&mut 0, &mut occs);
        occs
    }

    fn rec_into_occurrences(&self, id: &mut usize, occurrences: &mut Vec<Occurrence>) {
        for definition in self.definitions.values() {
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

            if let Some(references) = self.references.get(definition.identifier) {
                for reference in references {
                    occurrences.push(scip::types::Occurrence {
                        range: reference.node.to_scip_range(),
                        symbol: symbol.clone(),
                        ..Default::default()
                    });
                }
            }

            self.children
                .iter()
                .for_each(|c| c.occurrences_for_children(definition, symbol.as_str(), occurrences));
        }

        self.children
            .iter()
            .for_each(|c| c.rec_into_occurrences(id, occurrences));
    }

    fn occurrences_for_children(
        self: &Scope<'a>,
        def: &Definition<'a>,
        symbol: &str,
        occurrences: &mut Vec<Occurrence>,
    ) {
        if self.definitions.contains_key(def.identifier) {
            return;
        }

        for reference in &self.references {
            // if reference.identifier == def.identifier {
            //     occurrences.push(scip::types::Occurrence {
            //         range: reference.node.to_scip_range(),
            //         symbol: symbol.to_string(),
            //         ..Default::default()
            //     });
            // }
        }

        self.children
            .iter()
            .for_each(|c| c.occurrences_for_children(def, symbol, occurrences));
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum ScopeModifier {
    #[default]
    Local,
    Parent,
    Global,
}

#[derive(Debug)]
pub struct Definition<'a> {
    pub group: &'a str,
    pub identifier: &'a str,
    pub node: Node<'a>,
    pub range: ByteRange,
    pub scope_modifier: ScopeModifier,
}

#[derive(Debug)]
pub struct Reference<'a> {
    pub group: &'a str,
    pub identifier: &'a str,
    pub node: Node<'a>,
    pub range: ByteRange,
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
        let mut scope_modifier = None;

        for capture in m.captures {
            let capture_name = capture_names
                .get(capture.index as usize)
                .expect("capture indexes should always work");

            node = Some(capture.node);

            if capture_name.starts_with("definition") {
                assert!(definition.is_none(), "only one definition per match");
                definition = Some(capture_name);

                // Handle scope modifiers
                let properties = config.query.property_settings(m.pattern_index);
                for prop in properties {
                    if &(*prop.key) == "scope" {
                        match prop.value.as_deref() {
                            Some("global") => scope_modifier = Some(ScopeModifier::Global),
                            Some("parent") => scope_modifier = Some(ScopeModifier::Parent),
                            Some("local") => scope_modifier = Some(ScopeModifier::Local),
                            // TODO: Should probably error instead
                            Some(other) => panic!("unknown scope-testing: {}", other),
                            None => {}
                        }
                    }
                }
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
            let scope_modifier = scope_modifier.unwrap_or_default();
            definitions.push(Definition {
                range: ByteRange {
                    start: node.start_byte(),
                    end: node.end_byte(),
                },
                group,
                identifier,
                node,
                scope_modifier,
            });
        } else if let Some(group) = reference {
            let identifier = node.utf8_text(source_bytes).expect("utf8_text");
            references.push(Reference {
                range: ByteRange {
                    start: node.start_byte(),
                    end: node.end_byte(),
                },
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

    // Sort smallest to largest, so we can pop off the end of the list for the largest, first scope
    scopes.sort_by_key(|m| {
        (
            std::cmp::Reverse(m.range.start),
            m.range.end - m.range.start,
        )
    });

    dbg!(scopes.len());
    dbg!(definitions.len());
    dbg!(references.len());

    let capacity = definitions.len() + references.len();

    // Add all the scopes to our tree
    while let Some(m) = scopes.pop() {
        root.insert_scope(m);
    }

    while let Some(m) = definitions.pop() {
        root.insert_definition(m);
    }

    // TODO: Collapse these scopes, to reduce nesting.
    let mut matched_scopes = vec![];
    root.find_scopes_with(&mut matched_scopes);
    dbg!(matched_scopes.len());

    while let Some(m) = references.pop() {
        root.insert_reference(m);
    }

    // dbg!(&root);
    let occs = root.into_occurrences(capacity);

    // if true {
    //     return Ok(vec![]);
    // }

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

    #[test]
    fn test_can_do_functions() -> Result<()> {
        let mut config = crate::languages::go_locals();
        let source_code = include_str!("../testdata/funcs.go");
        let doc = parse_file_for_lang(&mut config, source_code)?;

        let dumped = dump_document(&doc, source_code);
        insta::assert_snapshot!(dumped);

        Ok(())
    }
}
