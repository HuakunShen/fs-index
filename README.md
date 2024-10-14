## Requirements

I need a rust function for indexing a given folder of files and subfolders.
The purpose is to get the file structure and size of each file and folders. 
Folder size is the sum of all files inside it, recursively.
The returned structure should be a tree.

Let's call the structure struct FileNode.
It should contain: 
- name
- size
- children
- type: file or directory

Here are a few extra requirements:
1. It should run as fast as possible, take advantage of parallelism and all cores available.
2. Make it serializable. The purpose is to be able to store it some where and load it back later.
3. Also provide a fuzzy search function for the tree
4. For directories containing a .gitignore file, it should ignore directories in .gitignore file. We still need to total folder size, but don't store the structure.