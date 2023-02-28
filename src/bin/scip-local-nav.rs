use std::{fs, path::Path};

use scip::{types::Document, write_message_to_file};
use scip_semantic::{languages::LocalConfiguration, locals::parse_tree};

fn recurse_dirs(config: &mut LocalConfiguration, root: &Path, dir: &Path) -> Vec<Document> {
    // TODO: Filtr

    let extension = "go";

    let mut documents = vec![];
    for entry in dir.read_dir().expect("is a valid directory") {
        let entry = entry.expect("is a valid entry");

        let path = entry.path();

        if path.is_dir() {
            documents.extend(recurse_dirs(config, root, &path));
        } else {
            match path.extension() {
                Some(ext) => {
                    if ext != extension {
                        continue;
                    }
                }
                None => continue,
            }

            let contents = fs::read_to_string(&path).expect("is a valid file");
            let tree = config
                .parser
                .parse(contents.as_bytes(), None)
                .expect("to parse the tree");

            let occs = parse_tree(config, &tree, contents.as_bytes()).expect("to get occurrences");

            let mut doc = Document::new();
            doc.language = "go".to_string();
            doc.relative_path = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            doc.occurrences = occs;
            doc.symbols = vec![];

            // All the symbols are local, so we don't want to do this.
            // doc.symbols = doc
            //     .occurrences
            //     .iter()
            //     .map(|o| scip::types::SymbolInformation {
            //         symbol: o.symbol.clone(),
            //         ..Default::default()
            //     })
            //     .collect();

            documents.push(doc);
        }
    }

    documents
}

fn main() {
    println!("scip-local-nav");

    let directory = Path::new("/home/tjdevries/sourcegraph/sourcegraph.git/main/");
    // let extension = "go";

    let mut index = scip::types::Index {
        metadata: Some(scip::types::Metadata {
            tool_info: Some(scip::types::ToolInfo {
                name: "scip-local-nav".to_string(),
                version: "0.0.1".to_string(),
                arguments: vec![],
                ..Default::default()
            })
            .into(),
            project_root: "file://".to_string() + directory.to_str().unwrap(),
            ..Default::default()
        })
        .into(),
        ..Default::default()
    };

    let mut config = scip_semantic::languages::go_locals();
    index
        .documents
        .extend(recurse_dirs(&mut config, directory, directory));

    println!("{:?}", index.documents.len());
    write_message_to_file(directory.join("index.scip"), index).expect("to write the file");
}
