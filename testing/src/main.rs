use std::fs;

// ⋚

fn add_two_integers(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let result = add_two_integers(5, 3);
    println!("The sum is: {}", result);
}
