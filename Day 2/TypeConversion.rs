/*
❌ This fails:
let x: i32 = 5;
let y: f64 = x;
*/

fn main() {
    let x: i32 = 5;
    let y: f64 = x as f64;

    println!("y = {}", y);
}

//Rust forces you to be explicit.


// Shadowing (VERY IMPORTANT)

// Shadowing ≠ mutability.


fn main() {
    let x = 5;
    let x = x + 1;
    let x = x * 2;

    println!("x = {}", x);
}



/*

Why Shadowing Is Powerful

Change type

Keep immutability

Cleaner logic
*/
