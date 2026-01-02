# 📅 **Day 2 — Data Types, Type System & Shadowing**

### 🎯 Day 2 Goals

By the end of today, you will:

* Understand **Rust’s type system**
* Know all **basic primitive data types**
* Understand **type inference vs explicit typing**
* Master **shadowing** (very Rust-specific, very powerful)

---

## 1️⃣ Why Rust Cares So Much About Types

Rust is:

* **Statically typed** → types known at compile time
* **Strongly typed** → no implicit unsafe conversions

This allows:

* Memory safety
* Zero-cost abstractions
* Very aggressive compiler checks

> In Rust, types are part of program correctness, not decoration.

---

## 2️⃣ Type Inference (Rust Figures It Out)

```rust
fn main() {
    let x = 10;
    let y = 3.14;

    println!("x = {}, y = {}", x, y);
}
```

### What Rust Infers

| Variable | Type                    |
| -------- | ----------------------- |
| `x`      | `i32` (default integer) |
| `y`      | `f64` (default float)   |

Rust chooses **safe defaults**:

* Integers → `i32`
* Floats → `f64`

---

## 3️⃣ Explicit Type Annotation

Sometimes you **must** specify types:

```rust
fn main() {
    let x: i64 = 100;
    let y: f32 = 2.5;

    println!("x = {}, y = {}", x, y);
}
```

### Syntax

```rust
let variable_name: Type = value;
```

---

## 4️⃣ Integer Types (Very Important)

Rust integers come in **sizes** and **signedness**.

### Signed Integers (can be negative)

| Type    | Size          |
| ------- | ------------- |
| `i8`    | 8-bit         |
| `i16`   | 16-bit        |
| `i32`   | 32-bit        |
| `i64`   | 64-bit        |
| `i128`  | 128-bit       |
| `isize` | CPU-dependent |

### Unsigned Integers (only positive)

| Type    | Size            |
| ------- | --------------- |
| `u8`    | 8-bit           |
| `u16`   | 16-bit          |
| `u32`   | 32-bit          |
| `u64`   | 64-bit          |
| `u128`  | 128-bit         |
| `usize` | memory indexing |

### Example

```rust
fn main() {
    let a: u8 = 255;
    let b: i8 = -128;

    println!("a = {}, b = {}", a, b);
}
```

⚠️ Overflow is **checked** in debug mode.

---

## 5️⃣ Floating-Point Types

```rust
fn main() {
    let x: f32 = 3.14;
    let y: f64 = 2.718281828;

    println!("x = {}, y = {}", x, y);
}
```

| Type  | Precision                  |
| ----- | -------------------------- |
| `f32` | single precision           |
| `f64` | double precision (default) |

---

## 6️⃣ Boolean Type

```rust
fn main() {
    let is_rust_fun: bool = true;

    println!("Is Rust fun? {}", is_rust_fun);
}
```

Only two values:

```rust
true
false
```

---

## 7️⃣ Character Type (`char`)

⚠️ **Not a byte!**

```rust
fn main() {
    let c: char = 'A';
    let emoji: char = '🦀';

    println!("{} {}", c, emoji);
}
```

* Uses **single quotes**
* Unicode scalar value
* 4 bytes internally

---

## 8️⃣ Numeric Operations

```rust
fn main() {
    let sum = 5 + 3;
    let diff = 10 - 4;
    let product = 6 * 7;
    let quotient = 10.0 / 3.0;
    let remainder = 10 % 3;

    println!("{}, {}, {}, {}, {}", sum, diff, product, quotient, remainder);
}
```

---

## 9️⃣ Type Conversion (Casting)

Rust **does not** auto-convert types.

❌ This fails:

```rust
let x: i32 = 5;
let y: f64 = x;
```

✅ Correct:

```rust
fn main() {
    let x: i32 = 5;
    let y: f64 = x as f64;

    println!("y = {}", y);
}
```

### Rule:

Rust forces you to **be explicit**.

---

## 🔟 Shadowing (VERY IMPORTANT)

Shadowing ≠ mutability.

```rust
fn main() {
    let x = 5;
    let x = x + 1;
    let x = x * 2;

    println!("x = {}", x);
}
```

### Output

```text
x = 12
```

### Why Shadowing Is Powerful

* Change **type**
* Keep immutability
* Cleaner logic

---

## 1️⃣1️⃣ Shadowing vs `mut`

### Using `mut`

```rust
let mut x = 5;
x = x + 1;
```

### Using shadowing

```rust
let x = 5;
let x = x + 1;
```

### Key Differences

| Feature            | `mut` | Shadowing |
| ------------------ | ----- | --------- |
| Change type        | ❌     | ✅         |
| Keeps immutability | ❌     | ✅         |
| Preferred in Rust  | ❌     | ✅         |

---

## 1️⃣2️⃣ Changing Types with Shadowing

```rust
fn main() {
    let spaces = "   ";
    let spaces = spaces.len();

    println!("spaces = {}", spaces);
}
```

❌ This would NOT work with `mut`

---

## 1️⃣3️⃣ Compile-Time Errors (Learn to Love Them)

Try this:

```rust
fn main() {
    let x = 10;
    x = 20;
}
```

Rust error messages:

* Clear
* Helpful
* Often suggest fixes

> Rust errors are documentation.

---

## 🧪 Practice Tasks (Very Important)

### ✅ Task 1

Create variables:

* age (`i32`)
* height (`f64`)
* is_student (`bool`)

Print them.

---

### ✅ Task 2

Use shadowing to:

1. Start with a number
2. Add 5
3. Convert it to `f64`

---

### ✅ Task 3

Try assigning a float to an integer **without casting** and observe the error.

---

## 🧠 Day 2 Key Takeaways

* Rust is strongly & statically typed
* Type inference is smart, but explicit typing matters
* No implicit type conversions
* Shadowing is safer than mutability
* Compiler errors teach you Rust

---



