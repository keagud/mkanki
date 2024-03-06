

use mkanki::mkanki::read_md_file;

fn main() -> mkanki::Result<()>{

    let test_file = concat!(env!("CARGO_MANIFEST_DIR"), "/test_assets/test.md");

    dbg!(read_md_file(test_file)?);
    println!("Hello, world!");

    Ok(())
}
