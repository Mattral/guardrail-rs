
# 📅 **Day 4 — Functions, Expressions & Semicolons**

### 🎯 Day 4 Goals

By the end of today, you will:

* Write and call functions confidently
* Understand parameters and return values
* Fully grasp **expressions vs statements**
* Understand **why semicolons matter** in Rust
* Read Rust code without guessing

---

## 1️⃣ Why Functions Matter in Rust

Functions in Rust:

* Are **strongly typed**
* Always have **explicit parameter types**
* Often return values implicitly

Rust treats functions as **expression builders**, not just procedures.

---

## 2️⃣ Defining a Function

```rust
fn greet() {
    println!("Hello from Rust!");
}
```

### Breakdown

* `fn` → function keyword
* `greet` → function name
* `()` → parameters (empty here)
* `{}` → function body

Call it:

```rust
fn main() {
    greet();
}
```

---

## 3️⃣ Function with Parameters

```rust
fn greet(name: &str) {
    println!("Hello, {}!", name);
}

fn main() {
    greet("Min");
}
```

### Key Rules

* Every parameter **must have a type**
* Rust does not guess parameter types

---

## 4️⃣ Function Returning a Value

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let sum = add(3, 4);
    println!("sum = {}", sum);
}
```

### Important Details

* `-> i32` → return type
* **No semicolon** after `a + b`
* Last expression is returned automatically

---

## 5️⃣ `return` Keyword (Optional but Allowed)

```rust
fn multiply(a: i32, b: i32) -> i32 {
    return a * b;
}
```

### When to Use `return`

* Early exit
* Conditional logic

Example:

```rust
fn abs(x: i32) -> i32 {
    if x >= 0 {
        x
    } else {
        -x
    }
}
```

---

## 6️⃣ Expressions vs Statements (CRITICAL CONCEPT)

### Expression

* Produces a value

```rust
5
x + 1
if true { 10 } else { 20 }
```

### Statement

* Performs action
* Does **not** return value

```rust
let x = 5;
println!("Hello");
```

---

## 7️⃣ Semicolons Change Meaning

This is an expression:

```rust
x + 1
```

This is a statement:

```rust
x + 1;
```

### Example

❌ This fails:

```rust
fn add_one(x: i32) -> i32 {
    x + 1;
}
```

✔️ Correct:

```rust
fn add_one(x: i32) -> i32 {
    x + 1
}
```

---

## 8️⃣ Blocks `{}` Are Expressions Too

```rust
fn main() {
    let x = {
        let a = 3;
        let b = 4;
        a + b
    };

    println!("x = {}", x);
}
```

### Why This Matters

* Scopes are values
* Enables functional-style logic

---

## 9️⃣ Functions Calling Functions

```rust
fn square(x: i32) -> i32 {
    x * x
}

fn cube(x: i32) -> i32 {
    x * square(x)
}

fn main() {
    println!("{}", cube(3));
}
```

---

## 🔟 Early Return with `return`

```rust
fn safe_divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        return 0;
    }
    a / b
}
```

---

## 1️⃣1️⃣ Multiple Parameters & Readability

```rust
fn calculate_area(width: f64, height: f64) -> f64 {
    width * height
}
```

Rust style:

* snake_case
* descriptive names

---

## 1️⃣2️⃣ Unit Return Type `()`

```rust
fn log_message(msg: &str) {
    println!("{}", msg);
}
```

This function returns `()` implicitly.

### `()` means:

> “nothing useful”

---

## 1️⃣3️⃣ Main Is Also a Function

```rust
fn main() -> () {
    println!("Main returns unit");
}
```

---

## 🧪 Practice Tasks (Strongly Recommended)

### ✅ Task 1

Write a function:

```text
fn is_even(n: i32) -> bool
```

---

### ✅ Task 2

Write a function that:

* Takes two numbers
* Returns the larger one
* Uses `if` as expression

---

### ✅ Task 3

Write a function that:

* Converts Celsius → Fahrenheit

---

## 🧠 Day 4 Key Takeaways

* Functions are expression-based
* Return values often don’t use `return`
* Semicolons matter **a lot**
* Blocks `{}` can produce values
* Rust favors clarity over convenience

---
