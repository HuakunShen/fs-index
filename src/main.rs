use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufRead, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use bytesize::ByteSize;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum NodeType {
    File,
    Directory,
    IgnoredDirectory,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileNode {
    name: String,
    size: u64,
    node_type: NodeType,
    children: Vec<FileNode>,
}

impl FileNode {
    fn new(name: String, size: u64, node_type: NodeType) -> Self {
        FileNode {
            name,
            size,
            node_type,
            children: Vec::new(),
        }
    }

    fn add_child(&mut self, child: FileNode) {
        self.size += child.size;
        self.children.push(child);
    }
}

fn read_gitignore(path: &Path) -> io::Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(path);
    let gitignore_path = path.join(".gitignore");
    if gitignore_path.exists() {
        builder.add(gitignore_path);
    }
    Ok(builder.build().unwrap())
}

fn calculate_ignored_size(path: &Path) -> io::Result<u64> {
    let mut total_size = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            total_size += metadata.len();
        } else if metadata.is_dir() {
            total_size += calculate_ignored_size(&entry.path())?;
        }
    }
    Ok(total_size)
}

fn index_folder(path: &Path, gitignore: &Gitignore) -> io::Result<FileNode> {
    let metadata = fs::metadata(path)?;
    let name = path.file_name().unwrap().to_string_lossy().into_owned();

    if metadata.is_file() {
        if gitignore.matched(path, false).is_ignore() {
            return Ok(FileNode::new(name, metadata.len(), NodeType::File));
        }
        Ok(FileNode::new(name, metadata.len(), NodeType::File))
    } else {
        let mut node = FileNode::new(name, 0, NodeType::Directory);
        let new_gitignore = read_gitignore(path)?;

        if new_gitignore.matched(path, true).is_ignore() {
            let size = calculate_ignored_size(path)?;
            return Ok(FileNode::new(
                path.to_string_lossy().into_owned(),
                size,
                NodeType::IgnoredDirectory,
            ));
        }

        let children: Vec<FileNode> = fs::read_dir(path)?
            .par_bridge()
            .filter_map(|entry| {
                entry
                    .ok()
                    .and_then(|e| index_folder(&e.path(), &new_gitignore).ok())
            })
            .collect();

        for child in children {
            node.add_child(child);
        }

        Ok(node)
    }
}

fn fuzzy_search(root: &FileNode, query: &str) -> Vec<String> {
    let matcher = SkimMatcherV2::default();
    let mut results = Vec::new();

    fn search_recursive(
        node: &FileNode,
        query: &str,
        matcher: &SkimMatcherV2,
        results: &mut Vec<String>,
        path: &mut Vec<String>,
    ) {
        if matcher.fuzzy_match(&node.name, query).is_some() {
            results.push(path.join("/"));
        }

        path.push(node.name.clone());
        for child in &node.children {
            search_recursive(child, query, matcher, results, path);
        }
        path.pop();
    }

    let mut path = Vec::new();
    search_recursive(root, query, &matcher, &mut results, &mut path);
    results
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <folder_path>", args[0]);
        return Ok(());
    }

    let folder_path = &args[1];
    let root_gitignore = read_gitignore(Path::new(folder_path))?;

    let start = Instant::now();
    let root = index_folder(Path::new(folder_path), &root_gitignore)?;
    let duration = start.elapsed();
    // Serialize the tree
    let serialized = serde_json::to_string_pretty(&root)?;
    fs::write("file_tree.json", serialized)?;
    // println!("{:#?}", root);
    println!("File tree has been indexed and saved to file_tree.json");
    println!("Time taken to index: {:?}", duration);
    println!("Total size: {}", ByteSize::b(root.size));

    // Example of fuzzy search
    let search_query = "example";
    let search_results = fuzzy_search(&root, search_query);
    println!(
        "Fuzzy search results for '{}': {:?}",
        search_query, search_results
    );

    Ok(())
}
