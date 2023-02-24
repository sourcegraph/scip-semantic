use tree_sitter::Language;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub fn rust() -> Language {
    tree_sitter_rust::language()
}

pub fn go() -> Language {
    tree_sitter_go::language()
}
