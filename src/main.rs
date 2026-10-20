use saveshelf::{list, restore, snapshot, verify, Result};
use std::{env, path::Path, process};

fn run(args: &[String]) -> Result<()> {
    if args.is_empty() || args[0] == "--help" {
        println!("SaveShelf\nsnapshot SOURCE --vault VAULT\nlist --vault VAULT\nverify SNAPSHOT\nrestore SNAPSHOT --output NEW_DIRECTORY\nRestore writes NEW_DIRECTORY/files and never overwrites existing destinations.");
        return Ok(());
    }
    match args[0].as_str() {
        "snapshot" if args.len() == 4 && args[2] == "--vault" => println!(
            "{}",
            snapshot(Path::new(&args[1]), Path::new(&args[3]))?.display()
        ),
        "list" if args.len() == 3 && args[1] == "--vault" => {
            for path in list(Path::new(&args[2]))? {
                println!("{}", path.display());
            }
        }
        "verify" if args.len() == 2 => {
            let m = verify(Path::new(&args[1]))?;
            println!(
                "Verified {} files and {} directories.",
                m.files.len(),
                m.directories.len()
            );
        }
        "restore" if args.len() == 4 && args[2] == "--output" => println!(
            "Restored verified files to {}",
            restore(Path::new(&args[1]), Path::new(&args[3]))?.display()
        ),
        _ => return Err("invalid command; use --help".into()),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(&env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("saveshelf: {error}");
        process::exit(2);
    }
}
