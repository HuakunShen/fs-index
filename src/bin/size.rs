use bytesize::ByteSize;
use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

fn calculate_folder_size(path: &Path) -> io::Result<u64> {
    let metadata = fs::metadata(path)?;

    if metadata.is_file() {
        return Ok(metadata.len());
    }

    fs::read_dir(path)?
        .par_bridge()
        .try_fold(
            || 0,
            |acc, entry| {
                let entry = entry?;
                let size = calculate_folder_size(&entry.path())?;
                Ok(acc + size)
            },
        )
        .try_reduce(|| 0, |a, b| Ok(a + b))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <folder_path>", args[0]);
        return;
    }

    let folder_path = Path::new(&args[1]);
    let start = Instant::now();
    match calculate_folder_size(folder_path) {
        Ok(size) => println!("Total size: {}", ByteSize::b(size)),
        Err(e) => eprintln!("Error: {}", e),
    }
    let duration = start.elapsed();
    println!("Time taken: {:?}", duration);
}
