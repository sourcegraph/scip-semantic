use tree_sitter::Node;

pub(crate) fn contains<'a>(scope: &Node<'a>, node: &Node<'a>) -> bool {
    scope.start_byte() <= node.start_byte() && scope.end_byte() >= node.end_byte()
}

#[allow(dead_code)]
pub(crate) fn walk_child(node: &Node, depth: usize) {
    let mut cursor = node.walk();

    node.children(&mut cursor).for_each(|child| {
        println!(
            "{}{:?} {} {}",
            " ".repeat(depth),
            child,
            child.start_position().column,
            child.end_position().column
        );

        walk_child(&child, depth + 1);
    });
}
