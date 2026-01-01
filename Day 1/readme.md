---

# 📅 **Day 1 — Rust Basics**

### 🎯 Today’s Goal

* Understand **what Rust is**
* Write your **first Rust program**
* Learn **variables and printing**

---

## 1️⃣ What is Rust?

Rust is:

* **Compiled** (fast like C/C++)
* **Memory-safe** (no segfaults, no GC)
* **Strict** (forces good habits early)

Rust prevents bugs **at compile time**, not runtime.

---

## 2️⃣ Installing Rust

Use the official installer:

```bash
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
```

Check installation:

```bash
rustc --version
cargo --version
```

---

## 3️⃣ Your First Rust Program

Create a file called `main.rs`

```rust
fn main() {
    println!("Hello, Rust!");
}
```

### 🔍 Explanation

* `fn` → defines a function
* `main` → program entry point
* `{}` → function body
* `println!` → **macro** (not a function)
* `!` → means macro
* `"Hello, Rust!"` → string literal

Run it:

```bash
rustc main.rs
./main
```

---

## 4️⃣ Variables in Rust

```rust
fn main() {
    let x = 5;
    println!("x is {}", x);
}
```

### Important Rules

* `let` → creates a variable
* Variables are **immutable by default**
* `{}` → placeholder for values

---

## 5️⃣ Mutability

This ❌ **will not compile**:

```rust
let x = 5;
x = 10;
```

Correct version:

```rust
fn main() {
    let mut x = 5;
    x = 10;
    println!("x is {}", x);
}
```

### Why Rust does this?

Immutability:

* Prevents accidental bugs
* Makes code easier to reason about
* Helps concurrency later

---

## 6️⃣ Printing Multiple Values

```rust
fn main() {
    let name = "Rust";
    let year = 2015;

    println!("{} was released in {}", name, year);
}
```

---

## 7️⃣ Basic Comments

```rust
// This is a single-line comment

/*
 This is
 a multi-line
 comment
*/
```

---

## 8️⃣ Very Small Practice (Do This)

```rust
fn main() {
    let mut age = 20;
    age = age + 1;

    println!("I am {} years old", age);
}
```

Try to:

* Remove `mut` → see the error
* Change numbers
* Add another variable

---

## 🧠 Key Takeaways (Day 1)

* Rust programs start in `main`
* `println!` is a macro
* Variables are immutable by default
* Compiler errors are **your friend**

---
