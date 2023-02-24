use tree_sitter::Node;

pub mod prelude {
    pub use super::ContainsNode;
}

pub trait ContainsNode {
    fn contains_node(&self, node: &Node) -> bool;
}

impl<'a> ContainsNode for Node<'a> {
    fn contains_node(&self, node: &Node) -> bool {
        self.start_byte() <= node.start_byte() && self.end_byte() >= node.end_byte()
    }
}

// #[allow(dead_code)]
// pub fn walk_child(node: &Node, depth: usize) {
//     let mut cursor = node.walk();
//
//     node.children(&mut cursor).for_each(|child| {
//         println!(
//             "{}{:?} {} {}",
//             " ".repeat(depth),
//             child,
//             child.start_position().column,
//             child.end_position().column
//         );
//
//         walk_child(&child, depth + 1);
//     });
// }
