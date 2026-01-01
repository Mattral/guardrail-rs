// Variables in Rust

fn main() {
    let x = 5;
    println!("x is {}", x);
}

/*
Important Rules

let → creates a variable

Variables are immutable by default

{} → placeholder for values
*/



// Mutability This ❌ will not compile:

//let x = 5;
//x = 10;


//Correct version:

fn main() {
    let mut x = 5;
    x = 10;
    println!("x is {}", x);
}


/*
Why Rust does this?

Immutability:

Prevents accidental bugs

Makes code easier to reason about

Helps concurrency later

*/

