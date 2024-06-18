use std::io::{self, Read};

// QUIK

fn main() {
    let mut raw = String::new();
    io::stdin()
        .read_to_string(&mut raw)
        .expect("failed to read io");
    println!("{}", raw);
}
