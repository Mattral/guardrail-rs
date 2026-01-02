fn main() {
    let x = 10;
    let y = 3.14;

    println!("x = {}, y = {}", x, y);
}

/*
What Rust Infers
Variable	Type
x	        i32 (default integer)
y	        f64 (default float)

Rust chooses safe defaults:
Integers → i32
Floats → f64
*/
