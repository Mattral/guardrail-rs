# 📅 **Day 5 — Tuples, Arrays & Slices**

### 🎯 Day 5 Goals

By the end of today, you will:

* Understand **tuples** vs **arrays**
* Know when to use each
* Learn **fixed-size memory concepts**
* Understand **slices (`&[T]`)**
* Read Rust data structures confidently

---

## 1️⃣ Why This Matters

Before ownership, you must understand:

* **How data is grouped**
* **Where size is known**
* **How Rust sees memory layouts**

Tuples, arrays, and slices form the **core data containers**.

---

## 2️⃣ Tuples — Grouping Different Types

### Basic Tuple

```rust
fn main() {
    let person = ("Min", 25, true);
    println!("{:?}", person);
}
```

### Key Properties

* Fixed size
* Can store **different types**
* Indexed access

---

## 3️⃣ Accessing Tuple Elements

```rust
fn main() {
    let person = ("Min", 25, true);

    println!("Name: {}", person.0);
    println!("Age: {}", person.1);
    println!("Student: {}", person.2);
}
```

Indexing starts from `0`.

---

## 4️⃣ Destructuring Tuples

```rust
fn main() {
    let person = ("Min", 25, true);

    let (name, age, is_student) = person;

    println!("{} is {} years old", name, age);
}
```

### Why This Is Powerful

* Clean extraction
* Avoids indexing
* Readable code

---

## 5️⃣ Returning Multiple Values with Tuples

```rust
fn calculate(x: i32, y: i32) -> (i32, i32) {
    (x + y, x * y)
}

fn main() {
    let (sum, product) = calculate(3, 4);
    println!("sum = {}, product = {}", sum, product);
}
```

Rust uses tuples instead of “out parameters”.

---

## 6️⃣ Arrays — Fixed Size, Same Type

```rust
fn main() {
    let numbers = [10, 20, 30, 40];
    println!("{:?}", numbers);
}
```

### Properties

* Fixed length
* Same type
* Stored on stack (usually)

---

## 7️⃣ Accessing Array Elements

```rust
fn main() {
    let numbers = [10, 20, 30];

    println!("{}", numbers[0]);
}
```

⚠️ Out-of-bounds access causes **panic**, not undefined behavior.

---

## 8️⃣ Explicit Array Type

```rust
fn main() {
    let numbers: [i32; 3] = [1, 2, 3];
}
```

Syntax:

```rust
[type; length]
```

---

## 9️⃣ Initialize Arrays Quickly

```rust
fn main() {
    let zeros = [0; 5];
    println!("{:?}", zeros);
}
```

Creates: `[0, 0, 0, 0, 0]`

---

## 🔟 Iterating Over Arrays (Correct Way)

```rust
fn main() {
    let numbers = [10, 20, 30];

    for n in numbers {
        println!("{}", n);
    }
}
```

✔️ Safe
✔️ Idiomatic Rust

---

## 1️⃣1️⃣ Slices — View Into Data (CRITICAL)

A **slice** is a **reference to part of a collection**.

```rust
fn main() {
    let numbers = [10, 20, 30, 40, 50];

    let slice = &numbers[1..4];

    println!("{:?}", slice);
}
```

### Output

```text
[20, 30, 40]
```

---

## 1️⃣2️⃣ Slice Rules

* Does **not own** data
* Has **length**
* Has **start pointer**
* Very cheap (no copy)

Type:

```rust
&[i32]
```

---

## 1️⃣3️⃣ Full Array Slice

```rust
fn main() {
    let numbers = [1, 2, 3];

    let whole = &numbers[..];
    println!("{:?}", whole);
}
```

---

## 1️⃣4️⃣ Slices in Functions

```rust
fn sum(slice: &[i32]) -> i32 {
    let mut total = 0;
    for x in slice {
        total += x;
    }
    total
}

fn main() {
    let numbers = [1, 2, 3, 4];
    println!("{}", sum(&numbers));
}
```

### Why Rust Prefers Slices

* Works with arrays & vectors
* No ownership transfer
* Maximum flexibility

---

## 1️⃣5️⃣ Tuple vs Array vs Slice

| Feature         | Tuple | Array | Slice   |
| --------------- | ----- | ----- | ------- |
| Fixed size      | ✅     | ✅     | ❌       |
| Same type       | ❌     | ✅     | ✅       |
| Own data        | ✅     | ✅     | ❌       |
| Stack allocated | ✅     | ✅     | ❌ (ref) |

---

## 🧪 Practice Tasks (Very Important)

### ✅ Task 1

Create a tuple representing:

* Name
* Age
* Height

Destructure and print values.

---

### ✅ Task 2

Write a function that:

* Takes an array slice
* Returns the maximum value

---

### ✅ Task 3

Create an array of 10 numbers

* Create a slice of first 5
* Print both

---

## 🧠 Day 5 Key Takeaways

* Tuples group **different types**
* Arrays are fixed-size, same-type
* Slices are **views**, not owners
* Rust prefers slices in function APIs
* Memory safety starts here

---
