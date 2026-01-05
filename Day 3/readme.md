# 📅 **Day 3 — Control Flow in Rust**

### 🎯 Day 3 Goals

By the end of today, you will:

* Master `if`, `else if`, `else`
* Understand why `if` is an **expression**
* Use all Rust loops: `loop`, `while`, `for`
* Learn `break` with values
* Use ranges and iteration safely

---

## 1️⃣ Control Flow in Rust (Big Idea)

In Rust:

* Control flow constructs **return values**
* Many things are **expressions**, not statements

> This enables safer, cleaner logic with fewer temporary variables.

---

## 2️⃣ `if` and `else`

### Basic Example

```rust
fn main() {
    let number = 7;

    if number > 5 {
        println!("Number is greater than 5");
    } else {
        println!("Number is 5 or less");
    }
}
```

### Important Rules

* Condition **must be `bool`**
* No implicit truthy/falsy values

❌ This is illegal:

```rust
if number { }
```

---

## 3️⃣ `else if`

```rust
fn main() {
    let number = 0;

    if number > 0 {
        println!("Positive");
    } else if number < 0 {
        println!("Negative");
    } else {
        println!("Zero");
    }
}
```

---

## 4️⃣ `if` as an Expression (Very Important)

```rust
fn main() {
    let condition = true;

    let number = if condition {
        10
    } else {
        20
    };

    println!("number = {}", number);
}
```

### Key Rules

* Both branches must return **same type**
* No semicolon on the returned value

❌ This fails:

```rust
let x = if true { 5 } else { "hello" };
```

---

## 5️⃣ Why Rust Does This

Instead of:

```rust
let x;
if condition {
    x = 10;
} else {
    x = 20;
}
```

Rust allows:

```rust
let x = if condition { 10 } else { 20 };
```

→ fewer bugs, clearer intent

---

## 6️⃣ `loop` — Infinite Loop

```rust
fn main() {
    let mut counter = 0;

    loop {
        counter += 1;
        println!("counter = {}", counter);

        if counter == 3 {
            break;
        }
    }
}
```

### When to Use

* Embedded systems
* Event loops
* Retry logic

---

## 7️⃣ `break` with a Value (Rust Feature)

```rust
fn main() {
    let mut counter = 0;

    let result = loop {
        counter += 1;

        if counter == 5 {
            break counter * 2;
        }
    };

    println!("result = {}", result);
}
```

### Output

```text
result = 10
```

⚠️ `loop` returns the value passed to `break`

---

## 8️⃣ `while` Loop

```rust
fn main() {
    let mut number = 3;

    while number != 0 {
        println!("{}", number);
        number -= 1;
    }

    println!("LIFTOFF!");
}
```

### Use Case

* Condition-based looping
* When you **don’t know iteration count upfront**

---

## 9️⃣ `for` Loop (Most Common)

```rust
fn main() {
    for i in 1..4 {
        println!("i = {}", i);
    }
}
```

### Output

```text
1
2
3
```

### Range Types

| Syntax  | Meaning |
| ------- | ------- |
| `1..4`  | 1 to 3  |
| `1..=4` | 1 to 4  |

---

## 🔟 Iterating Over Arrays

```rust
fn main() {
    let numbers = [10, 20, 30];

    for num in numbers {
        println!("{}", num);
    }
}
```

✔️ Safe
✔️ No index errors
✔️ Preferred Rust style

---

## 1️⃣1️⃣ Using `_` (Ignore Variable)

```rust
fn main() {
    for _ in 0..3 {
        println!("Hello");
    }
}
```

Used when loop variable isn’t needed.

---

## 1️⃣2️⃣ `continue` Keyword

```rust
fn main() {
    for i in 1..=5 {
        if i == 3 {
            continue;
        }
        println!("{}", i);
    }
}
```

### Output

```text
1
2
4
5
```

---

## 1️⃣3️⃣ Nested Loops

```rust
fn main() {
    for i in 1..=3 {
        for j in 1..=2 {
            println!("i = {}, j = {}", i, j);
        }
    }
}
```

---

## 1️⃣4️⃣ Labeled Loops (Advanced but Useful)

```rust
fn main() {
    'outer: for i in 1..=3 {
        for j in 1..=3 {
            if i == 2 && j == 2 {
                break 'outer;
            }
            println!("i = {}, j = {}", i, j);
        }
    }
}
```

Breaks **outer loop**, not inner.

---

## 🧪 Practice Tasks (Do These)

### ✅ Task 1

Write a program that:

* Checks if a number is even or odd
* Uses `if` as an expression

---

### ✅ Task 2

Use `loop` to:

* Count from 1
* Stop when number reaches 7
* Return the square of the number

---

### ✅ Task 3

Use `for` loop to:

* Print numbers from 10 to 1 (reverse)

---

## 🧠 Day 3 Key Takeaways

* `if` is an expression
* Rust loops are safe and expressive
* `loop` can return values
* `for` is preferred over manual indexing
* No truthy/falsy — only `bool`

---
