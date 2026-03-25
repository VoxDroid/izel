# ⬡ IZEL — PROJECT OVERVIEW
### *"Only One" — A Systems Programming Language Unlike Any Other*

> **Izel** (from Nahuatl: *izel* — "unique", "one of a kind", "only one") is a compiled,
> multi-paradigm systems programming language built entirely in Rust. It does not follow.
> It does not imitate. It forges its own path.

---

## Table of Contents

1.  [Identity & Vision](#1-identity--vision)
2.  [What Makes Izel Truly Unique](#2-what-makes-izel-truly-unique)
3.  [Design Philosophy](#3-design-philosophy)
4.  [Lexical Structure & Notation](#4-lexical-structure--notation)
5.  [Variables & Bindings](#5-variables--bindings)
6.  [Primitive & Compound Types](#6-primitive--compound-types)
7.  [Functions — `forge`](#7-functions--forge)
8.  [Structures — `shape`](#8-structures--shape)
9.  [Enumerations — `scroll`](#9-enumerations--scroll)
10. [Traits & Interfaces — `weave`](#10-traits--interfaces--weave)
11. [Pattern Matching — `branch`](#11-pattern-matching--branch)
12. [Control Flow](#12-control-flow)
13. [Closures & Lambdas](#13-closures--lambdas)
14. [Iterators & Pipelines](#14-iterators--pipelines)
15. [Error Handling — Cascade Errors](#15-error-handling--cascade-errors)
16. [Generics & Parametric Polymorphism](#16-generics--parametric-polymorphism)
17. [The Effect System](#17-the-effect-system)
18. [Witness Types](#18-witness-types)
19. [Temporal Constraints](#19-temporal-constraints)
20. [Memory Zones](#20-memory-zones)
21. [Duality Types](#21-duality-types)
22. [Compile-Time Evaluation — `echo`](#22-compile-time-evaluation--echo)
23. [Modules & Visibility — `ward`](#23-modules--visibility--ward)
24. [Macros & Meta-Programming](#24-macros--meta-programming)
25. [Concurrency & Async — `flow` / `tide`](#25-concurrency--async--flow--tide)
26. [Raw Blocks & FFI — `raw` / `bridge`](#26-raw-blocks--ffi--raw--bridge)
27. [Architecture Overview](#27-architecture-overview)
28. [Compiler Pipeline (Detailed)](#28-compiler-pipeline-detailed)
29. [Type System (Formal)](#29-type-system-formal)
30. [Memory Model](#30-memory-model)
31. [Standard Library](#31-standard-library)
32. [Toolchain](#32-toolchain)
33. [Project Directory Structure](#33-project-directory-structure)
34. [Milestones & Roadmap](#34-milestones--roadmap)
35. [Dependencies](#35-dependencies)
36. [Contributing Guidelines](#36-contributing-guidelines)
37. [Language Specification Index](#37-language-specification-index)

---

## 1. Identity & Vision

### The Name

**Izel** is a Nahuatl word meaning *"unique"*, *"only one"*, *"the singular one"*. This is not just a name — it is a contract with every developer who uses it. Izel will not be another C clone. It will not be Rust with different keywords. It will not be Go with extra features. It is its own complete idea, built from first principles, shaped by the conviction that systems programming can be radically more expressive, more safe, and more artful.

### The Mission

To create a compiled systems programming language that:

- Gives developers **absolute control** over hardware, memory, and execution — without sacrificing readability or safety.
- Introduces genuinely **novel language constructs** (Effect System, Witness Types, Temporal Constraints, Memory Zones, Duality Types) that solve problems other languages work around.
- Has a **syntax and grammar that is entirely its own** — not derived from C, not borrowing from Python, not inheriting Go's minimalism. Every keyword, every sigil, every structural choice is intentional and original.
- Treats the **developer experience as a first-class feature** — error messages that teach, tooling that understands intent, documentation that ships with the compiler.
- **Compiles to native machine code** via LLVM with zero runtime overhead and zero garbage collector.

### Core Promises

| Promise | Meaning |
|---------|---------|
| **Zero GC** | No garbage collector. Memory is managed through ownership, borrowing, and memory zones. |
| **Zero Hidden Cost** | Every allocation, every copy, every effect is visible. Nothing is done behind your back. |
| **Zero Compromise** | You can write high-level functional code and low-level pointer arithmetic in the same language without mode-switching. |
| **One Canonical Form** | Code formatted by `izel fmt` always looks the same. There is one Izel style. |
| **One Truth** | The type system, effect system, and borrow checker collectively eliminate entire classes of bugs at compile time. |

---

## 2. What Makes Izel Truly Unique

Izel is not merely "another systems language." Below is a comparison of its novel features against the current landscape:

| Feature | Izel | Rust | C++ | Go | Zig | Swift |
|---------|------|------|-----|----|-----|-------|
| Native Effect System | YES | NO | NO | NO | NO | NO |
| Witness Types | YES | NO | NO | NO | NO | NO |
| Temporal Constraints (Pre/Post) | YES | NO | NO | NO | NO | NO |
| Memory Zones | YES | NO | NO | NO | NO | NO |
| Duality Types | YES | NO | NO | NO | NO | NO |
| Cascade Error Propagation | YES | Partial | NO | NO | NO | NO |
| Pipeline Operator (`\|>`) | YES | NO | NO | NO | NO | NO |
| Compile-time Echo Blocks | YES | Partial | Partial | NO | YES | NO |
| Mutable Sigil Syntax (`~`) | YES | NO | NO | NO | NO | NO |
| Unique Keyword Grammar | YES | NO | NO | NO | NO | NO |
| Borrow Checker | YES | YES | NO | NO | NO | NO |
| Zero GC | YES | YES | YES | NO | YES | NO |
| Full Multi-Paradigm | YES | Partial | YES | NO | NO | YES |
| First-class Async | YES | YES | NO | YES | NO | YES |

### The Seven Pillars of Uniqueness

**1. The Effect System** — Functions declare what they *do* to the world, not just what they *return*. Effects like `!io`, `!panic`, `!alloc`, `!net` are part of the function's type signature. The compiler tracks effect propagation and enforces effect boundaries. No more "this function logs to a file, but nothing in its signature tells you that."

**2. Witness Types** — A `Witness<P>` is a zero-cost compile-time proof that some predicate `P` holds. You cannot construct an invalid witness without `raw`. This eliminates entire categories of runtime panics (division by zero, index out of bounds, null dereference) by encoding validity into the type.

**3. Temporal Constraints** — Functions can declare `@requires` (preconditions) and `@ensures` (postconditions) that are verified at compile time for constant inputs and wrapped into runtime checks (with zero overhead in release builds) for dynamic inputs. This is design-by-contract, native to the language.

**4. Memory Zones** — A `zone` block defines a named, scoped memory region. All allocations inside it are tracked and freed when the zone ends — regardless of how many functions, loops, or branches were executed inside it. It is an arena allocator that lives in the syntax.

**5. Duality Types** — A `dual` shape is a type that has two perfectly symmetric representations (e.g., serializer/deserializer, encoder/decoder, reader/writer) derived from a single definition. Write once, get both.

**6. The `~` Mutability Sigil** — Mutability in Izel is not a keyword modifier; it is a sigil prefix on the binding itself. `~x` is a mutable variable. `~Point { ... }` is a mutable struct literal. This makes mutability immediately visible at any point in code — you never need to scroll back to the declaration.

**7. The Cascade Error System** — Errors in Izel carry an automatic context chain. When an error propagates using `!`, each call site automatically appends its source location and a configurable message to the error's context stack. By the time an error reaches the surface, it has a full, human-readable chain of causation — like a stack trace, but for the error's *meaning*, not just its *location*.

---

## 3. Design Philosophy

### The Twelve Principles of Izel

**1. Visibility above all.** Every side effect, every allocation, every mutation is syntactically visible. You cannot hide complexity in Izel.

**2. Make the right thing the easy thing.** The default behavior (immutable bindings, safe memory, effect-tracked functions) is always the correct behavior. Unsafe power is available but requires explicit opt-in.

**3. Errors are citizens.** Errors are not exceptions. They are not magic control flow. They are values that carry meaning, context, and causation. They are handled with the same expressive power as any other value.

**4. The type system is your ally.** Types in Izel should capture as much intent as possible. A `NonZero<i32>` says more than an `i32`. A `Sorted<Vec<T>>` says more than a `Vec<T>`. The compiler understands these distinctions.

**5. Zero-cost means zero-cost.** If a feature is "zero-cost," it means the compiler produces identical machine code to the equivalent hand-written version. Not approximately. Exactly.

**6. Composition, not inheritance.** `weave` (trait/interface) + implementation is the primary mechanism for polymorphism. Deep class hierarchies are an antipattern in Izel.

**7. The pipeline is a first-class construct.** Data transformations are expressed as pipelines using `|>`. This is not sugar — the compiler understands and optimizes pipelines as fused loops.

**8. Compile time is powerful time.** Anything that can be resolved at compile time should be. `echo` blocks, `@requires`/`@ensures`, `Witness` types, and constant evaluation push work from runtime to compile time wherever possible.

**9. One canonical form.** There is one way to format Izel code. `izel fmt` is deterministic and non-configurable. There are no style debates.

**10. Fail loudly, fail early.** Izel never silently proceeds when something is wrong. Implicit truncation, implicit coercion, undefined behavior — none of these exist. When something goes wrong, the compiler tells you, loudly and precisely.

**11. The unsafe boundary is a wall, not a door.** `raw` blocks exist. They are necessary. But they are clearly delineated, auditable, and described. Code outside a `raw` block is provably safe.

**12. Naming is semantics.** The keywords of Izel are chosen to convey meaning. `forge` means to create something with craft and heat. `shape` means to define the form of something. `weave` means to interlock behaviors. `ward` means to protect a boundary. `scroll` means a list of named things. `echo` means to evaluate at a distance (compile time). These are not arbitrary — they encode the semantics of the construct.

---

## 4. Lexical Structure & Notation

### 4.1 File Extension

Izel source files use the `.iz` extension. Izel module bundles use `.izm`.

### 4.2 Character Set

Izel source files are encoded in UTF-8. Identifiers may contain Unicode letters and digits (following UAX #31). Keywords are ASCII only.

### 4.3 Comments

```izel
// Single-line comment

/~ Multi-line comment.
   Can span many lines. ~/

/// Doc comment — attached to the next declaration.
/// Supports Markdown and embedded code blocks.
/// ```izel
/// let x = 42;
/// ```

//! Crate/module-level doc comment (placed at the top of a file).
```

### 4.4 Sigils

Izel uses sigils for syntactic clarity:

| Sigil | Meaning | Example |
|-------|---------|---------|
| `~` | Mutable binding | `~x = 5`, `~Point { x: 1.0 }` |
| `!` | Effect annotation / error propagation | `forge f() !io`, `result!` |
| `@` | Compile-time attribute / constraint | `@requires(n > 0)`, `@inline` |
| `\|>` | Pipeline (pipe-forward) | `x \|> double \|> print` |
| `::` | Path separator | `std::io::println` |
| `->` | Return type / function arrow | `forge f() -> i32` |
| `=>` | Branch arm | `Red => "red"` |
| `..` | Range (exclusive) | `0..10` |
| `..=` | Range (inclusive) | `0..=10` |
| `..` | Struct update / spread | `Shape { ..other }` |
| `&` | Immutable borrow | `&val` |
| `&~` | Mutable borrow | `&~val` |
| `*` | Dereference | `*ptr` |
| `?` | Optional unwrap (in `given` context) | `x?` |
| `#` | Attribute macro | `#[derive(Debug)]` |

### 4.5 Keywords

Izel's complete keyword list:

```
forge    shape    scroll   weave    ward     echo
branch   given    else     loop     each     while
break    next     give     let      raw      bridge
flow     tide     zone     dual     seek     catch
draw     open     hidden   pkg      pure     sole
self     Self     true     false    nil      as
in       of       is       not      and      or
comptime static   extern   type     alias    impl
```

### 4.6 Reserved Future Keywords

```
proof    contract  phantom  region   effect   session
atomic   quantum   lattice  reflect  derive
```

### 4.7 Literals

```izel
// Integer literals
42          // decimal
0xFF        // hexadecimal
0o77        // octal
0b1010_1010 // binary (underscores allowed)
42_000_000  // decimal with separators

// Float literals
3.14
2.0e10
1.5e-3

// String literals
"Hello, World!"
"Escaped: \n \t \\ \""
"Unicode: \u{1F600}"

// Raw strings
r"No \n escape here"
r#"Can contain "quotes" inside"#

// Interpolated strings (backtick)
`Hello, {name}!`
`Result: {x + y}`

// Byte literals
b'A'        // u8 value 65
b"bytes"    // &[u8]

// Character literals
'a'
'\n'
'\u{263A}'

// Boolean
true
false

// Nil (no value — used with optional types)
nil
```

---

## 5. Variables & Bindings

### 5.1 Immutable Bindings (`let`)

By default, all bindings in Izel are immutable. Reassignment is a compile error.

```izel
let x = 42
let name: str = "Izel"
let pi: f64 = 3.14159265358979
```

### 5.2 Mutable Bindings (`~`)

The `~` sigil marks a binding as mutable. It appears as a prefix on the binding name, not as a keyword modifier. This makes mutability visible at every use site, not just the declaration.

```izel
~x = 42         // type inferred
~x: i32 = 42    // explicit type
~x += 1         // reassignment allowed

// The ~ travels with the name in function signatures too:
forge increment(~n: &~i32) {
    *n += 1
}
```

### 5.3 Shadowing

Izel supports name shadowing — a new binding with the same name replaces the old one in the current scope.

```izel
let x = 5
let x = x * 2       // shadows previous x; type can change
let x: str = "now a string"
```

### 5.4 Destructuring

```izel
// Tuple destructuring
let (a, b) = (1, 2)

// Struct destructuring
let Point { x, y } = point

// Array/slice destructuring
let [first, second, ..rest] = array

// Nested
let (Point { x, y }, z) = (point, 42)

// With rename
let Point { x: px, y: py } = point
```

### 5.5 Constants and Statics

```izel
// Compile-time constant (no address, inlined everywhere)
const MAX_SIZE: usize = 4096

// Static variable (has an address, lives for program duration)
static COUNTER: u64 = 0

// Mutable static (requires raw block to access safely)
static ~LOG_LEVEL: u8 = 0
```

---

## 6. Primitive & Compound Types

### 6.1 Numeric Types

| Type | Size | Description |
|------|------|-------------|
| `i8` | 1B | Signed integer |
| `i16` | 2B | Signed integer |
| `i32` | 4B | Signed integer (default integer literal type) |
| `i64` | 8B | Signed integer |
| `i128` | 16B | Signed integer |
| `isize` | ptr | Pointer-sized signed integer |
| `u8` | 1B | Unsigned integer (byte) |
| `u16` | 2B | Unsigned integer |
| `u32` | 4B | Unsigned integer |
| `u64` | 8B | Unsigned integer |
| `u128` | 16B | Unsigned integer |
| `usize` | ptr | Pointer-sized unsigned integer |
| `f32` | 4B | IEEE 754 single-precision float |
| `f64` | 8B | IEEE 754 double-precision float (default float literal type) |

### 6.2 Other Primitives

| Type | Description |
|------|-------------|
| `bool` | `true` or `false` |
| `char` | A Unicode scalar value (32-bit) |
| `str` | A UTF-8 string slice (unsized, always behind `&`) |
| `()` | Unit type — the type of an expression with no meaningful value |
| `!` | Never type — the type of an expression that never returns |

### 6.3 Compound Types

```izel
// Tuple — ordered, fixed, heterogeneous
let pair: (i32, str) = (1, "one")

// Array — fixed-size, homogeneous
let arr: [i32; 5] = [1, 2, 3, 4, 5]
let zeros = [0u8; 256]

// Slice — dynamically-sized view
let slice: &[i32] = &arr[1..3]

// Optional type — ?T is sugar for Option<T>
let maybe: ?i32 = nil
let some:  ?i32 = 42

// Raw pointers (require raw blocks)
let ptr:  *i32   = &x as *i32
let mptr: *~i32  = &~x as *~i32
```

### 6.4 The `?T` Optional Type

`?T` is first-class syntax for `Option<T>`. Optional chaining and the null-coalescing operator `??` work naturally with it.

```izel
forge find_user(id: u64) -> ?User { ... }

// Optional chaining
let city = find_user(1)?.address?.city

// Null coalescing
let name = find_user(1)?.name ?? "anonymous"
```

---

## 7. Functions — `forge`

The keyword `forge` is used to define functions. The word means to create something with deliberate craft — appropriate for named, callable units of computation.

### 7.1 Basic Syntax

```izel
forge name(param1: Type1, param2: Type2) -> ReturnType {
    body
}
```

The last expression in a function body is its implicit return value. Use `give` for explicit early returns.

```izel
forge add(a: i32, b: i32) -> i32 {
    a + b       // implicit return
}

forge early(x: i32) -> i32 {
    given x < 0 { give 0 }    // explicit early return
    x * 2
}
```

### 7.2 `give` — The Return Statement

```izel
forge abs(x: i32) -> i32 {
    given x < 0 { give -x }
    give x
}
```

### 7.3 Named Parameters & Defaults

```izel
forge connect(
    host: &str,
    port: u16 = 8080,
    timeout: u32 = 30,
) -> Result<Connection> !net {
    // ...
}

// Call with named parameters
connect(host: "localhost", timeout: 60)
connect(host: "prod.server.com", port: 443)
```

### 7.4 Variadic Functions

```izel
forge sum(..values: i32) -> i32 {
    values.iter() |> fold(0, bind |acc, x| acc + x)
}

sum(1, 2, 3, 4, 5)   // => 15
```

### 7.5 `pure` Functions

A `pure` function has no effects: no I/O, no global mutation, no allocation, no panic. The compiler verifies this statically. Pure functions can be evaluated at compile time by the `echo` system.

```izel
pure forge square(x: i32) -> i32 { x * x }

pure forge fibonacci(n: u32) -> u64 {
    branch n {
        0 => 0,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

### 7.6 Method Syntax (on shapes)

```izel
shape impl Point {
    forge into_tuple(self) -> (f64, f64) {      // consumes self
        (self.x, self.y)
    }

    forge length(&self) -> f64 {                 // borrows self
        (self.x * self.x + self.y * self.y).sqrt()
    }

    forge scale(&~self, factor: f64) {           // mutably borrows self
        ~self.x *= factor
        ~self.y *= factor
    }
}
```

### 7.7 Function Overloading

Scoped to a `shape impl` or `weave` implementation only (free functions use generics):

```izel
shape impl Vector2 {
    forge new(x: f64, y: f64) -> Self { Self { x, y } }
    forge new(v: (f64, f64))  -> Self { Self { x: v.0, y: v.1 } }
}
```

---

## 8. Structures — `shape`

`shape` defines a named product type. The word conveys the structural, geometric nature of defining a type's form.

### 8.1 Basic Shape

```izel
shape Point {
    x: f64,
    y: f64,
}

shape Person {
    name: String,
    age: u32,
    email: ?String,
}
```

### 8.2 Instantiation & Mutation

```izel
let p = Point { x: 1.0, y: 2.0 }          // immutable
~p = Point { x: 1.0, y: 2.0 }             // mutable
~p.x = 5.0                                  // field mutation allowed

// Struct update syntax
let p2 = Point { x: 10.0, ..p }
```

### 8.3 Tuple Shapes & Unit Shapes

```izel
shape Color(u8, u8, u8)     // tuple shape
let red = Color(255, 0, 0)
let r = red.0

shape Marker                // zero-sized unit shape
```

### 8.4 Packed & Aligned Shapes

```izel
#[packed]
shape RawHeader {
    magic: u32,
    version: u16,
    flags: u8,
}

#[align(64)]
shape HotData {
    value: u64,
    timestamp: u64,
}
```

### 8.5 Visibility

```izel
open shape PublicPoint {
    open x: f64,
    open y: f64,
}

shape InternalBuffer {
    hidden data: Vec<u8>,
    open capacity: usize,
}
```

---

## 9. Enumerations — `scroll`

`scroll` defines a sum type — a named set of variants. The word evokes a list of named things, like a scroll of records.

### 9.1 Basic & Data-Carrying Variants

```izel
scroll Direction { North, South, East, West }

scroll Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Triangle(f64, f64, f64),
    Point,
}
```

### 9.2 Recursive Scrolls

```izel
scroll Tree<T> {
    Leaf(T),
    Node {
        value: T,
        left:  Box<Tree<T>>,
        right: Box<Tree<T>>,
    },
}
```

### 9.3 Scroll Methods

```izel
shape impl Shape {
    forge area(&self) -> f64 {
        branch self {
            Shape::Circle { radius }               => std::math::PI * radius * radius,
            Shape::Rectangle { width, height }     => width * height,
            Shape::Triangle(a, b, c) => {
                let s = (a + b + c) / 2.0
                (s * (s-a) * (s-b) * (s-c)).sqrt()
            },
            Shape::Point => 0.0,
        }
    }
}
```

---

## 10. Traits & Interfaces — `weave`

`weave` defines an interface — a named set of behaviors. The word evokes interlocking threads, which is exactly what trait-based polymorphism does.

### 10.1 Defining & Implementing a Weave

```izel
weave Drawable {
    forge draw(&self)
    forge area(&self) -> f64
    forge perimeter(&self) -> f64

    // Default implementation
    forge describe(&self) -> String {
        `Shape with area {self.area():.2}`
    }
}

shape Circle { radius: f64 }

weave Drawable for Circle {
    forge draw(&self) {
        std::io::println(`Circle r={self.radius}`)
    }
    forge area(&self) -> f64 { std::math::PI * self.radius * self.radius }
    forge perimeter(&self) -> f64 { 2.0 * std::math::PI * self.radius }
}
```

### 10.2 Weave as Type Bounds

```izel
// Static dispatch — monomorphized, zero-cost
forge print_area<T: Drawable>(shape: &T) {
    std::io::println(`Area: {shape.area()}`)
}

// Dynamic dispatch — vtable at runtime
forge print_area_dyn(shape: &dyn Drawable) {
    std::io::println(`Area: {shape.area()}`)
}
```

### 10.3 Associated Types & Inheritance

```izel
weave Container {
    type Item
    forge get(&self, idx: usize) -> ?&Self::Item
    forge len(&self) -> usize
}

weave Shape: Drawable + Clone {
    forge bounding_box(&self) -> Rect
}
```

### 10.4 Operator Overloading via Weaves

| Operator | Weave | Method |
|----------|-------|--------|
| `+` | `std::ops::Add` | `forge add(self, rhs: Rhs) -> Output` |
| `-` | `std::ops::Sub` | `forge sub(self, rhs: Rhs) -> Output` |
| `*` | `std::ops::Mul` | `forge mul(self, rhs: Rhs) -> Output` |
| `/` | `std::ops::Div` | `forge div(self, rhs: Rhs) -> Output` |
| `==` | `std::cmp::Eq` | `forge eq(&self, other: &Self) -> bool` |
| `<`, `>` | `std::cmp::Ord` | `forge cmp(&self, other: &Self) -> Ordering` |
| `[]` | `std::ops::Index` | `forge index(&self, idx: Idx) -> &Self::Output` |
| `\|>` | `std::ops::Pipe` | `forge pipe(self, f: F) -> F::Output` |

---

## 11. Pattern Matching — `branch`

`branch` is Izel's exhaustive pattern matching construct. It is more powerful than a `switch` and more readable than nested `if`/`else`.

### 11.1 Basic Matching

```izel
branch value {
    0       => "zero",
    1..=9   => "single digit",
    10..=99 => "double digit",
    _       => "large",
}
```

### 11.2 Destructuring, Guards, Multi-Pattern

```izel
branch point {
    Point { x: 0.0, y: 0.0 } => "origin",
    Point { x, y: 0.0 }      => `x-axis at {x}`,
    Point { x: 0.0, y }      => `y-axis at {y}`,
    Point { x, y }            => `at ({x}, {y})`,
}

// Guards
branch n {
    x given x < 0       => "negative",
    0                   => "zero",
    x given x % 2 == 0  => "positive even",
    _                   => "positive odd",
}

// Multi-pattern arms
branch c {
    'a' | 'e' | 'i' | 'o' | 'u' => "vowel",
    'a'..='z' | 'A'..='Z'        => "consonant",
    '0'..='9'                    => "digit",
    _                            => "other",
}
```

---

## 12. Control Flow

### 12.1 `given` / `else` (Conditionals)

`given` replaces `if`. The word conveys "given that this condition holds."

```izel
given x > 0 {
    std::io::println("positive")
} else given x < 0 {
    std::io::println("negative")
} else {
    std::io::println("zero")
}

// As expression
let label = given x > 0 { "pos" } else { "non-pos" }

// Conditional unwrap
given let Some(user) = find_user(id) {
    greet(user)
}
```

### 12.2 Loops

```izel
// Infinite loop
loop {
    given done() { break }
}

// While loop
while i < 10 { ~i += 1 }

// For-each
each item in collection { process(item) }
each (i, item) in collection.enumerate() { ... }
each i in 0..10 { ... }

// Loop labels
'outer: each i in 0..10 {
    each j in 0..10 {
        given i * j > 50 { break 'outer }
    }
}
```

### 12.3 `break`, `next` (continue)

```izel
each x in 0..100 {
    given x % 2 == 0 { next }    // skip evens
    given x > 50 { break }        // stop
    process(x)
}
```

---

## 13. Closures & Lambdas

Closures in Izel are introduced with `bind`. The word conveys capturing and binding variables from the surrounding scope.

```izel
let double = bind |x: i32| x * 2
let add    = bind |a: i32, b: i32| -> i32 { a + b }

// Capture by borrow
let threshold = 42
let above = bind |x: i32| x > threshold

// Capture by move
let name = String::from("Izel")
let greet = bind move || std::io::println(`Hello, {name}!`)

// As parameters
forge apply<T, R>(f: bind(T) -> R, val: T) -> R { f(val) }
apply(bind |x: i32| x * x, 5)   // => 25

// Returning closures
forge make_adder(n: i32) -> bind(i32) -> i32 {
    bind move |x| x + n
}
```

---

## 14. Iterators & Pipelines

Izel's `|>` pipeline operator and iterator combinators are a primary programming model. The compiler **fuses adjacent pipeline stages into single loops** at the MIR level — this is not sugar, it is a verified optimization.

### 14.1 The `|>` Operator

`x |> f` is equivalent to `f(x)`. It enables left-to-right data flow with no overhead.

```izel
let result = 5 |> double |> square |> to_string
// equivalent to: to_string(square(double(5)))

let evens_squared = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    |> filter(bind |&x| x % 2 == 0)
    |> map(bind |x| x * x)
    |> take(3)
    |> collect::<Vec<i32>>()
// => [4, 16, 36]  (fused into one loop at compile time)
```

### 14.2 Full Combinator List (std::iter)

| Combinator | Description |
|-----------|-------------|
| `map(f)` | Transform each element |
| `filter(pred)` | Keep matching elements |
| `filter_map(f)` | Map and filter in one pass |
| `flat_map(f)` | Map to iterators, flatten |
| `flatten()` | Flatten nested iterators |
| `fold(init, f)` | Reduce to single value |
| `scan(init, f)` | Like fold, but yields intermediates |
| `take(n)` | First n elements |
| `skip(n)` | Skip first n elements |
| `take_while(pred)` | Take while predicate holds |
| `skip_while(pred)` | Skip while predicate holds |
| `zip(other)` | Pair elements from two iterators |
| `enumerate()` | Attach zero-based index |
| `chain(other)` | Concatenate two iterators |
| `peekable()` | Allow non-consuming peek |
| `cloned()` | Clone each element |
| `collect::<C>()` | Gather into any collection type |
| `count()` | Count elements |
| `sum()` / `product()` | Arithmetic aggregation |
| `min()` / `max()` | Extrema |
| `any(pred)` / `all(pred)` | Boolean short-circuit checks |
| `find(pred)` / `position(pred)` | First match |
| `partition(pred)` | Split into two |
| `chunks(n)` / `windows(n)` | Sliced sub-sequences |

### 14.3 Custom Iterators

```izel
shape Fibonacci { ~a: u64, ~b: u64 }

weave Iterator for Fibonacci {
    type Item = u64
    forge next(&~self) -> ?u64 {
        let result = self.a
        let next   = self.a + self.b
        ~self.a = self.b
        ~self.b = next
        Some(result)
    }
}

let fib10 = Fibonacci { a: 0, b: 1 } |> take(10) |> collect::<Vec<_>>()
```

---

## 15. Error Handling — Cascade Errors

### 15.1 Result<T, E> and Error Propagation

```izel
forge parse_port(s: &str) -> Result<u16, ParseError> {
    let n = s.parse::<u32>()!   // ! propagates on Err
    given n > 65535 { give Err(ParseError::OutOfRange(n)) }
    Ok(n as u16)
}
```

### 15.2 The `!` Cascade Propagator

Unlike Rust's `?`, Izel's `!` automatically **enriches** the error with file, line, and an optional message, building a chain of causation:

```izel
forge load_config(path: &str) -> Result<Config> !io {
    let text   = std::fs::read_to_string(path)! or "reading config file"
    let config = Config::parse(&text)!           or "parsing config"
    Ok(config)
}

// If read_to_string fails, the error chain contains:
//   [0] load_config at config.iz:3 — "reading config file"
//   [1] std::fs::read_to_string at fs.iz:88
//   [2] TcpStream::connect at net.iz:244 — "connection refused"
```

### 15.3 `seek` / `catch` — Inline Handling

```izel
// Default value on error
let port = seek parse_port("abc") catch _ => 8080

// Match specific error kinds
let conn = seek connect(host, port)
    catch NetworkError::Timeout => retry(host, port)
    catch NetworkError::Refused => give Err(ServiceDown)
    catch e                     => give Err(e)
```

### 15.4 Custom Error Types

```izel
#[error]
scroll AppError {
    Io     { source: std::io::Error, path: String },
    Parse  { msg: String, line: u32 },
    NotFound { resource: String },
}

weave Display for AppError {
    forge fmt(&self, f: &~Formatter) -> FormatResult {
        branch self {
            AppError::Io { path, .. }       => write!(f, `IO error on: {path}`),
            AppError::Parse { msg, line }   => write!(f, `Parse error line {line}: {msg}`),
            AppError::NotFound { resource } => write!(f, `Not found: {resource}`),
        }
    }
}
```

---

## 16. Generics & Parametric Polymorphism

```izel
// Generic function
forge max<T: Ord>(a: T, b: T) -> T {
    given a >= b { a } else { b }
}

// Generic shape
shape Pair<A, B> {
    first: A,
    second: B,
}

shape impl<A, B> Pair<A, B> {
    forge swap(self) -> Pair<B, A> {
        Pair { first: self.second, second: self.first }
    }
}

// Multiple bounds + where clause
forge deserialize<T>(data: &[u8]) -> Result<T>
where
    T: Decode,
    T::Error: Display,
{
    T::decode(data)!
}

// Const generics
shape Matrix<T, const ROWS: usize, const COLS: usize> {
    data: [[T; COLS]; ROWS],
}
let m: Matrix<f64, 3, 3> = Matrix::identity()
```

---

## 17. The Effect System

This is one of Izel's most distinctive features. Every function in Izel has an **effect set** — a compile-time-known declaration of every side effect it may perform.

### 17.1 Built-in Effects

| Effect | Meaning |
|--------|---------|
| `!io` | File, stdout, or stderr I/O |
| `!net` | Network I/O |
| `!alloc` | Heap memory allocation |
| `!panic` | May call panic / abort |
| `!unsafe` | Contains or calls `raw` blocks |
| `!time` | Reads the system clock |
| `!rand` | Reads from a random source |
| `!env` | Reads environment variables |
| `!ffi` | Calls across an FFI boundary |
| `!thread` | Spawns or joins threads |

### 17.2 Declaring Effects

```izel
forge read_file(path: &str) -> Result<String> !io, !alloc {
    std::fs::read_to_string(path)
}

// No effects declared — compiler verifies this
forge pure_compute(x: i32) -> i32 {
    x * x + 1
}
```

### 17.3 Effect Propagation

Effects are propagated transitively. If `f` calls `g !io`, then `f` must also declare `!io` (or contain `g`'s call in an effect boundary).

### 17.4 Effect Boundaries

An effect boundary is a point where effects are contained and cannot escape the caller:

```izel
// Runs f in an isolated I/O context. Caller sees no !io effect.
let output = io::capture(bind || {
    std::io::println("Captured output")
    42
})
// output: CaptureResult { value: 42, stdout: "Captured output\n" }
```

### 17.5 Effect-Based Testing

Because effects are in the type, you can inject test doubles at the type level — zero mocking frameworks needed:

```izel
weave Logger {
    forge log(&self, msg: &str) !io
}

forge process<L: Logger>(data: &[u8], logger: &L) !io {
    logger.log("Processing...")
    // ...
}

// In tests: use a NoOpLogger that satisfies Logger but emits nothing
```

---

## 18. Witness Types

A `Witness<P>` is a zero-sized, zero-cost type that carries compile-time proof that predicate `P` holds. Witnesses cannot be constructed outside designated proof functions or `raw` blocks.

### 18.1 Built-in Witnesses

```izel
// NonZero<T> — value is non-zero
let n: NonZero<i32> = NonZero::new(42)!
let n: NonZero<i32> = NonZero::assert(42)   // panics if zero

// InBounds<usize> — index is valid for a slice
let idx: InBounds<usize> = slice.check_index(5)!

// Sorted<T> — slice is sorted
let data: Sorted<Vec<i32>> = vec.into_sorted()
```

### 18.2 Using Witnesses to Eliminate Runtime Checks

```izel
// Without Witness — check every call
forge divide(a: i32, b: i32) -> Result<i32, DivError> {
    given b == 0 { give Err(DivError::Zero) }
    Ok(a / b)
}

// With Witness — proof in the type; zero runtime overhead
forge divide(a: i32, b: NonZero<i32>) -> i32 {
    a / b.value()
}
```

### 18.3 Custom Witnesses

```izel
shape IsPositive  // unit shape as predicate name

forge prove_positive(n: i32) -> Result<Witness<IsPositive>, ()> {
    given n > 0 { Ok(Witness::new()) }
    else         { Err(()) }
}

forge sqrt_positive(n: i32, _proof: Witness<IsPositive>) -> f64 {
    (n as f64).sqrt()   // safe — proof already established
}
```

---

## 19. Temporal Constraints

Temporal constraints are first-class `@requires` (preconditions) and `@ensures` (postconditions) on functions. The compiler verifies them statically for constant inputs and emits optimized runtime checks in debug builds. Release builds elide them by default.

```izel
@requires(n >= 0, "n must be non-negative")
forge factorial(n: u64) -> u64 {
    given n == 0 { give 1 }
    n * factorial(n - 1)
}

@requires(lo <= hi, "range must be valid")
@ensures(result >= lo and result <= hi, "result in range")
forge clamp(val: f64, lo: f64, hi: f64) -> f64 {
    given val < lo { give lo }
    given val > hi { give hi }
    val
}

// Shape-level invariants
#[invariant(self.width > 0.0 and self.height > 0.0)]
shape Rect {
    width: f64,
    height: f64,
}
// Compiler checks that all Rect methods preserve this invariant
```

---

## 20. Memory Zones

A `zone` block defines a named, scoped memory region backed by an arena allocator. Everything allocated within a zone is freed when the zone ends — deterministically, regardless of control flow.

```izel
// Basic usage
zone temp {
    ~buf = Vec::<u8>::with_capacity_in(1024, zone::allocator())
    buf.extend_from_slice(input)
    process(trim_whitespace(&buf))
}   // buf freed here

// Named nested zones
zone request {
    let headers = parse_headers(raw)
    zone render {
        let tpl = load_template("index.html")
        send_response(render_template(tpl, &headers))
    }   // tpl freed
}   // headers freed

// Zone allocator passed to standard collections
zone batch {
    let alloc = batch::allocator()
    let results: Vec<_, _> = items
        |> map(bind |item| transform_in(item, alloc))
        |> collect()
    commit(&results)
}
```

Zone allocations **cannot escape** their zone — the borrow checker enforces this:

```izel
let escaped: &str
zone temp {
    let s = String::from_in("hello", zone::allocator())
    escaped = &s   // ERROR: `s` does not live long enough
}
```

---

## 21. Duality Types

A `dual` shape is a type with two perfectly symmetric representations derived from one definition. The canonical use case is serialization — write the encoding once, get decoding for free. The compiler derives the inverse and proves the round-trip law: `decode(encode(x)) == x`.

```izel
dual shape JsonFormat<T: Schema> {
    forge encode(&self, val: &T) -> JsonValue
    forge decode(&self, raw: &JsonValue) -> Result<T>
}

dual shape BinaryProtocol<T: Packed> {
    forge write(&self, val: &T, sink: &~impl Write) -> Result<()> !io
    forge read(&self,  source: &~impl Read)          -> Result<T> !io
}
```

The compiler automatically verifies the round-trip law:
- For `pure` duals: verified statically as a compile-time proof.
- For effectful duals: emitted as a `#[test]` that runs in `izel test`.

---

## 22. Compile-Time Evaluation — `echo`

`echo` blocks execute at compile time. They generate types, constants, and code available to the rest of the program. `echo` is the `comptime` of Izel, but with a richer code-generation API.

```izel
// Constant generation
echo {
    const PRIMES: [u64; 10] = sieve_of_eratosthenes(100)
}

// Type generation
echo {
    type SmallStr = [u8; 24]
}

// Code generation — derive a Builder for any shape
shape Person { name: String, age: u32, email: ?String }
echo { derive_builder!(Person) }

let p = Person::builder()
    .name("Alice")
    .age(30)
    .build()!

// Conditional compilation
echo {
    given cfg::os() == "windows" {
        alias PathSep = WindowsPathSep
    } else {
        alias PathSep = UnixPathSep
    }
}
```

---

## 23. Modules & Visibility — `ward`

`ward` defines a module — a named boundary protecting and grouping declarations. The word conveys guarding a perimeter.

### 23.1 Defining & Nesting Wards

```izel
ward math {
    open const PI: f64 = 3.14159265358979

    open pure forge sqrt(x: f64) -> f64 {
        std::math::sqrt(x)
    }

    // Private helper
    forge newton_step(x: f64, g: f64) -> f64 {
        (g + x / g) / 2.0
    }
}

ward graphics {
    ward geometry { open shape Point { ... } }
    ward color    { open shape Rgb   { ... } }
}
```

### 23.2 `draw` — Import Statement

`draw` is the import keyword. It pulls names from a ward into the current scope.

```izel
draw std::io
draw std::collections::HashMap
draw std::io::{println, eprintln}
draw math::*                        // wildcard (use sparingly)
draw super::helpers
```

### 23.3 File-Based Wards

Each `.iz` file is implicitly a ward named after the file:

```
src/
  main.iz          → ward main
  config.iz        → ward config
  network/
    mod.iz         → ward network
    tcp.iz         → ward network::tcp
    udp.iz         → ward network::udp
```

---

## 24. Macros & Meta-Programming

### 24.1 Declarative Macros

```izel
macro vec![..items] {
    {
        ~v = Vec::new()
        $(v.push($items);)*
        v
    }
}

let v = vec![1, 2, 3]
```

### 24.2 `#[derive(...)]` — Built-in Derivable Weaves

| Weave | Derives |
|-------|---------|
| `Debug` | Formatted debug output |
| `Display` | User-facing string |
| `Clone` / `Copy` | Deep clone / bitwise copy |
| `Eq` / `PartialEq` | `==` and `!=` |
| `Ord` / `PartialOrd` | Comparison operators |
| `Hash` | Hashability |
| `Default` | Zero-value constructor |
| `Serialize` / `Deserialize` | JSON/binary codec |
| `Builder` | Builder pattern |
| `Error` | Error type boilerplate |

### 24.3 Attribute Macros

```izel
#[test]
forge test_add() { assert!(add(2, 3) == 5) }

#[bench]
forge bench_sort(b: &~Bencher) {
    b.iter(bind || { ~v = large_vec(); v.sort() })
}

#[inline(always)]
forge hot(x: i32) -> i32 { x * 2 + 1 }

#[deprecated(since = "1.2.0", note = "Use new_fn")]
forge old_fn() { ... }
```

---

## 25. Concurrency & Async — `flow` / `tide`

### 25.1 Threads & Channels

```izel
let handle = thread::spawn(bind || heavy_computation())
let result = handle.join()!

let (tx, rx) = chan::<i32>::new()
thread::spawn(bind move || { each i in 0..10 { tx.send(i) } })
each val in rx { std::io::println(`received: {val}`) }
```

### 25.2 `flow` / `tide` — Async Functions & Await

`flow` marks a function as asynchronous. `tide` is the await operator — the word evokes waiting for the tide to come in, a natural non-blocking pause.

```izel
flow forge fetch_user(id: u64) -> Result<User> !net {
    let resp = tide http::get(`https://api.example.com/users/{id}`)!
    let user = tide resp.json::<User>()!
    Ok(user)
}

// Concurrent await
flow forge load_dashboard() -> Dashboard !net {
    let (user, posts, notifs) = tide::join(
        fetch_user(current_id()),
        fetch_posts(current_id()),
        fetch_notifs(current_id()),
    )!
    Dashboard { user, posts, notifs }
}
```

### 25.3 Atomic Types

```izel
let counter = Atomic::<u64>::new(0)
counter.fetch_add(1, Ordering::SeqCst)
let val = counter.load(Ordering::Relaxed)
```

---

## 26. Raw Blocks & FFI — `raw` / `bridge`

### 26.1 `raw` Blocks

Everything involving raw pointer arithmetic or unsafe operations requires a `raw` block. Code outside `raw` is provably safe. Every `raw` block must have a `SAFETY:` comment documenting its invariants.

```izel
/~ SAFETY: ptr is a valid, non-null, aligned pointer to at least len bytes.
   No other reference to this memory exists. ~/
#[unsafe]
forge read_bytes(ptr: *u8, len: usize) -> &[u8] {
    raw { std::slice::from_raw_parts(ptr, len) }
}

raw {
    let ptr: *mut u8 = std::alloc::alloc_raw(1024)
    given ptr.is_null() { panic!("OOM") }
    std::ptr::write(ptr, 0xFF)
    std::alloc::free_raw(ptr)
}
```

### 26.2 `bridge` — C/C++ FFI

```izel
bridge "C" {
    forge malloc(size: usize) -> *mut u8
    forge free(ptr: *mut u8)
    forge memcpy(dst: *mut u8, src: *u8, n: usize) -> *mut u8
    static errno: i32
}
```

### 26.3 Inline Assembly

```izel
raw {
    let result: u64
    asm!(
        "mov {0}, rsp",
        out(reg) result,
        options(nostack)
    )
    std::io::println(`Stack pointer: {result:#x}`)
}
```

---

## 27. Architecture Overview

```
Source (.iz files)
       |
       v
+------------------+
|   izel_lexer     |  UTF-8 source => Token stream (lossless, with spans)
+--------+---------+
         |
         v
+------------------+
|   izel_parser    |  Tokens => Lossless CST (all trivia preserved)
+--------+---------+
         |
         v
+------------------+
|  izel_ast_lower  |  CST => Canonical desugared AST
+--------+---------+
         |
         v
+------------------+
|  izel_resolve    |  Name resolution, module graph, symbol table
+--------+---------+
         |
         v
+------------------+
|  izel_typeck     |  HM type inference + effect checking +
|                  |  witness verification + contract checking
+--------+---------+
         |
         v
+------------------+
|  izel_borrow     |  Ownership + borrow checking (NLL) +
|                  |  lifetime inference + zone escape analysis
+--------+---------+
         |
         v
+------------------+
|  izel_hir        |  Monomorphization + echo evaluation +
|                  |  dual elaboration + macro expansion
+--------+---------+
         |
         v
+------------------+
|  izel_mir        |  Three-address SSA + CFG + drop elaboration +
|                  |  zone cleanup insertion
+--------+---------+
         |
         v
+------------------+
|  izel_opt        |  Constant folding/propagation, DCE, inlining,
|                  |  TCO, LICM, pipeline fusion, escape analysis,
|                  |  SROA, GVN, effect-based purity optimization
+--------+---------+
         |
         v
+------------------+
|  izel_codegen    |  Optimized MIR => LLVM IR (via inkwell)
+--------+---------+
         |
         v
+------------------+
|  LLVM Backend    |  LLVM IR => object files (x86_64 / aarch64 / riscv64 / wasm32)
+--------+---------+
         |
         v
+------------------+
|  Linker (lld)    |  Object files => ELF / Mach-O / PE / WASM binary
+------------------+
```

---

## 28. Compiler Pipeline (Detailed)

### 28.1 Lexer (`izel_lexer`)

- Hand-written DFA (deterministic finite automaton), zero external lexer dependencies.
- Produces a flat `Vec<Token>` with full span info (byte offset, line, column).
- Every character maps to exactly one token (lossless, trivia preserved).
- Token types: `Keyword`, `Ident`, `Literal` (6 sub-kinds), `Punct`, `Sigil`, `Comment`, `Whitespace`, `Eof`.
- Fuzz targets maintained for lexer crash safety.

### 28.2 Parser (`izel_parser`)

- Recursive descent with Pratt parsing for expressions (operator precedence).
- Produces a **lossless Concrete Syntax Tree (CST)** — every token retained. Enables formatter and LSP to reconstruct source exactly.
- Resilient error recovery: collects all parse errors before reporting, never panics.
- Precedence table (low to high):

| Level | Operators |
|-------|-----------|
| 1 | `\|>` |
| 2 | `or` |
| 3 | `and` |
| 4 | `not` |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=`, `is` |
| 6 | `\|` (bitwise or) |
| 7 | `^` (xor) |
| 8 | `&` (bitwise and) |
| 9 | `<<`, `>>` |
| 10 | `+`, `-` |
| 11 | `*`, `/`, `%` |
| 12 | Unary `-`, `not`, `~`, `*`, `&`, `&~` |
| 13 | `as` (cast), `!` (propagate) |
| 14 | `()`, `[]`, `.`, `::` |

### 28.3 AST Lowering

Desugars the following constructs into their canonical AST forms:

- `x!` → `branch x { Ok(v) => v, Err(e) => give Err(e.cascade(here!())) }`
- `?T` → `Option<T>`
- `x ?? default` → `x.unwrap_or(default)`
- `` `Hello, {name}` `` → `format!("Hello, {}", name)`
- `x?.y` → `given let Some(t) = x { t.y } else { None }`
- `each x in iter {}` → `iter.__into_iter().__next()` loop

### 28.4 Name Resolution

- Two-pass: collects all top-level declarations first (forward references), then resolves use sites.
- Handles `draw`, re-exports, wildcards, shadowing, `self`/`super`/`ward` paths.
- Detects: undefined names, ambiguous imports, unused imports, cyclic imports.

### 28.5 Type Checker (`izel_typeck`)

- Algorithm W (Hindley-Milner) extended with:
  - Structural subtyping for `dyn Weave` objects.
  - Associated types on `weave` definitions.
  - Const generics.
  - Effect unification: effects are unified as ordered sets.
- Effect checking: each call site's required effects must be declared by the caller.
- Witness checking: `Witness<P>` construction gated to proof functions and `raw` blocks.
- Temporal constraint verification: `@requires`/`@ensures` symbolically verified for constant inputs.

### 28.6 Borrow Checker (`izel_borrow`)

- Non-Lexical Lifetimes (NLL) based on the CFG.
- `&~T` mutable borrows: uniqueness enforced — no simultaneous aliasing.
- Memory zone escape analysis: no references escape their zone.
- Error messages provide actionable suggestions (explain the borrow, suggest a clone or move).

### 28.7 HIR (`izel_hir`)

- Monomorphizes generics: each instantiation gets a fully specialized copy.
- Evaluates `echo` blocks and constant expressions.
- Elaborates `dual` shapes into forward and inverse implementations.
- Expands `#[derive(...)]` macros into explicit weave implementations.

### 28.8 MIR (`izel_mir`)

- Three-address SSA (Static Single Assignment) form.
- Explicit basic blocks with phi nodes.
- Drop elaboration: RAII destructors inserted at each scope exit.
- Zone cleanup: deterministic free-all inserted at zone exit points.
- `here!()` cascade calls lowered to source location constants.

### 28.9 Optimizer (`izel_opt`)

| Pass | Description |
|------|-------------|
| Constant Folding | Evaluate `pure` expressions at compile time |
| Constant Propagation | Propagate known SSA constants through the CFG |
| Dead Code Elimination | Remove unreachable blocks and unused values |
| Dead Store Elimination | Remove writes never subsequently read |
| Inlining | Inline small functions by callee cost heuristic |
| Loop Invariant Code Motion | Hoist loop-invariant computations |
| Tail Call Optimization | Guaranteed TCO — all tail-recursive calls become loops |
| Pipeline Fusion | Fuse adjacent `.map().filter()` chains into single loops |
| Escape Analysis | Stack-promote heap allocations that don't escape |
| Effect Optimization | Elide redundant effect checks for provably pure calls |
| SROA | Split aggregates into independent scalars |
| GVN | Global value numbering — eliminate redundant computations |

### 28.10 Code Generation (`izel_codegen`)

- Uses `inkwell` (safe Rust bindings for LLVM 17+).
- MIR basic blocks map 1:1 to LLVM IR basic blocks.
- Platform ABI mapping: System V AMD64, AAPCS64, Win64.
- SIMD intrinsics lowered to LLVM vector intrinsics.
- DWARF debug info (all builds); stripped in `--opt=release`.
- Supported targets: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`, `riscv64gc-unknown-linux-gnu`, `wasm32-unknown-unknown`.

---

## 29. Type System (Formal)

### 29.1 Type Kinds

| Kind | Examples | Description |
|------|---------|-------------|
| `Type` | `i32`, `Point`, `?str` | Concrete types inhabitable by values |
| `Type -> Type` | `Vec`, `Option`, `Box` | Type constructors |
| `Type, Type -> Type` | `Result`, `HashMap` | Binary type constructors |
| `Nat -> Type` | `[T; N]`, `Matrix<T, R, C>` | Const-parameterized types |
| `Effect -> Type` | `forge() !io` | Effect-parameterized function types |

### 29.2 Subtyping Rules

- `T` is a subtype of `dyn Weave` if `T` implements `Weave`.
- `&T` is covariant in `T`.
- `&~T` is invariant in `T`.
- `forge(T) -> U` is contravariant in `T`, covariant in `U`.
- Effect sets are ordered by subset: `!io` < `!io, !net`.

### 29.3 Inference

Bidirectional type checking:
- **Checking mode**: expected type flows inward (annotated let bindings, function arguments).
- **Synthesis mode**: type is derived from the expression (unannotated `let x = 5`).

### 29.4 Coherence (Orphan Rule)

A `weave W for T` implementation is only permitted if either `W` or `T` is defined in the current package. This guarantees globally unique implementations and prevents conflicting behavior.

---

## 30. Memory Model

### 30.1 Ownership Rules

1. Every value has exactly one owner.
2. When the owner goes out of scope, the value is dropped (RAII).
3. Ownership can be moved or borrowed.

```izel
let a = String::from("hello")  // a owns the string
let b = a                       // ownership moves to b; a is invalid
let c = &b                      // c borrows from b; b still owns
```

### 30.2 Borrowing Rules

| Borrow Kind | Syntax | Rules |
|------------|--------|-------|
| Immutable | `&T` | Many allowed simultaneously; no mutation |
| Mutable | `&~T` | At most one at a time; no simultaneous immutable borrows |

### 30.3 RAII & Drop

```izel
weave Drop {
    forge drop(&~self)
}

shape FileHandle { fd: i32 }
weave Drop for FileHandle {
    forge drop(&~self) {
        raw { libc::close(self.fd) }
    }
}
// FileHandle::drop() called automatically at end of scope
```

### 30.4 Allocator Parameterization

All heap-allocating types are parameterized by an allocator:

```izel
shape Vec<T, A: Allocator = GlobalAlloc> { ... }

let v1 = Vec::<i32>::new()
let v2 = Vec::<i32, _>::new_in(my_arena.allocator())
```

The `Allocator` weave:

```izel
weave Allocator {
    forge alloc(&self, layout: Layout) -> Result<*~u8, AllocError>
    forge dealloc(&self, ptr: *~u8, layout: Layout)
    forge realloc(&self, ptr: *~u8, old: Layout, new_size: usize) -> Result<*~u8, AllocError>
}
```

### 30.5 Memory Regions Summary

| Mechanism | Location | Lifetime |
|-----------|----------|---------|
| Local bindings | Stack | Scope |
| `Box<T>` | Heap | RAII (Drop) |
| `Vec<T>` | Heap | RAII (Drop) |
| `zone` allocations | Heap arena | Zone exit |
| `raw` allocations | Anywhere | Programmer's responsibility |
| `static` variables | Data segment | Program lifetime |

---

## 31. Standard Library

### Core (no `!alloc` needed)

| Ward | Contents |
|------|----------|
| `std::prim` | Primitive type methods and constants |
| `std::ops` | Operator weaves (`Add`, `Sub`, `Mul`, `Pipe`, etc.) |
| `std::cmp` | `Eq`, `Ord`, `Ordering` |
| `std::iter` | `Iterator` weave and all combinators |
| `std::option` | `Option<T>` methods |
| `std::result` | `Result<T, E>` methods and `Cascade` error |
| `std::convert` | `From`, `Into`, `TryFrom`, `TryInto` |
| `std::fmt` | `Display`, `Debug`, `Formatter`, `format!` |
| `std::mem` | `size_of`, `align_of`, `transmute`, `drop` |
| `std::ptr` | Raw pointer utilities |
| `std::slice` | Slice methods |
| `std::str` | String slice methods |
| `std::range` | Range types |
| `std::marker` | `Copy`, `Send`, `Sync`, `Sized`, `Unpin` |

### Allocation & Collections

| Ward | Contents |
|------|----------|
| `std::alloc` | `Allocator`, `GlobalAlloc`, `Layout`, raw alloc/free |
| `std::boxed` | `Box<T>` |
| `std::string` | `String`, `StringBuilder` |
| `std::vec` | `Vec<T, A>` |
| `std::collections` | `HashMap`, `BTreeMap`, `HashSet`, `VecDeque`, `BinaryHeap` |
| `std::arc` | `Arc<T>` (atomically ref-counted) |
| `std::rc` | `Rc<T>` (non-thread-safe ref-counted) |
| `std::cell` | `Cell<T>`, `RefCell<T>` (interior mutability) |

### I/O & OS

| Ward | Contents |
|------|----------|
| `std::io` | `println`, `eprintln`, stdin/stdout/stderr, `Read`, `Write`, `Seek` |
| `std::fs` | `read_to_string`, `write`, `copy`, `create_dir`, `DirEntry` |
| `std::path` | `Path`, `PathBuf` |
| `std::env` | `args`, `vars`, `current_dir` |
| `std::os` | OS-specific extensions |
| `std::ffi` | `CStr`, `CString`, FFI helpers |

### Concurrency

| Ward | Contents |
|------|----------|
| `std::thread` | `spawn`, `join`, `sleep`, `park` |
| `std::sync` | `Mutex`, `RwLock`, `Condvar`, `Barrier`, `Once` |
| `std::atomic` | `Atomic<T>`, `Ordering` |
| `std::chan` | `Sender`, `Receiver`, multi-producer multi-consumer |
| `std::async` | Async runtime, `flow!` executor, `tide::join`, `tide::select` |

### Math, Hash, Codec

| Ward | Contents |
|------|----------|
| `std::math` | Trig, exp, log, `PI`, `E`, `INFINITY`, `NAN` |
| `std::hash` | `Hash`, `Hasher`, `DefaultHasher` |
| `std::crypt` | BLAKE3, SHA-2, constant-time comparison |
| `std::codec` | Base64, hex encoding/decoding |
| `std::json` | JSON via `dual` (round-trip law enforced) |

### Testing

| Ward | Contents |
|------|----------|
| `std::test` | `#[test]`, `assert!`, `assert_eq!`, `should_panic!` |
| `std::bench` | `#[bench]`, `Bencher`, `black_box` |
| `std::mock` | Mockable weave stubs for effect testing |

---

## 32. Toolchain

### `izelc` — Compiler Binary

```
izelc [OPTIONS] <file.iz>

  -o <path>              Output path
  --target <triple>      Cross-compilation target
  --opt <level>          0 | 1 | 2 | 3 | s | z
  --emit <type>          tokens | cst | ast | hir | mir | llvm-ir | asm | obj | exe
  --debug                Emit DWARF debug information
  --no-std               Exclude standard library
  --check-effects        Enforce strict effect annotations
  --check-contracts      Enable @requires/@ensures at runtime
  --keep-witnesses       Retain witness checks in release builds
  --lto                  Link-time optimization
  --strip                Strip debug symbols
  --target-cpu <cpu>     CPU model (e.g. native, x86-64-v3)
  --error-format <fmt>   human | json | short
  --edition <year>       Language edition (default: 2025)
```

### `izel` — Package Manager & Build System

```
izel new <name> [--lib | --bin | --workspace]
izel build       [--release] [--target <triple>]
izel run         [-- <args>]
izel test        [filter] [--threads <n>]
izel bench       [filter]
izel check
izel fmt         [--check]
izel lint
izel doc         [--open]
izel add <pkg>   [@<version>] [--dev]
izel remove <pkg>
izel update
izel publish
izel clean
izel tree
izel audit
```

### `Izel.toml` — Project Manifest

```toml
[package]
name        = "myapp"
version     = "0.1.0"
authors     = ["Your Name <you@example.com>"]
edition     = "2025"
description = "My application"
license     = "MIT"

[features]
default  = ["logging"]
logging  = ["dep:izel-log"]

[dependencies]
izel-http = "2.1"
izel-json = "1.4"
izel-log  = { version = "0.8", optional = true }

[dev-dependencies]
izel-mock = "0.3"
izel-prop = "0.5"

[profile.debug]
opt        = 0
debug      = true
contracts  = true     // @requires/@ensures active

[profile.release]
opt        = 3
lto        = true
strip      = true
contracts  = false    // elided (override with --keep-witnesses)

[[bin]]
name = "myapp"
path = "src/main.iz"
```

### `izel-lsp` — Language Server (LSP 3.17)

| Feature | Description |
|---------|-------------|
| Diagnostics | Real-time errors, warnings, hints |
| Hover | Type signatures, doc comments, effect annotations |
| Completion | Types, methods, fields, imports |
| Go-to-definition | Any declaration in workspace or std |
| Find references | All use sites of a symbol |
| Rename | Semantic rename across workspace |
| Code actions | Quick-fixes for common diagnostics |
| Inlay hints | Inferred types, lifetimes, effects |
| Semantic tokens | Full syntax highlighting data |
| Format | Full-document and range formatting |

### `izel-fmt` — Formatter

Deterministic, opinionated, non-configurable. One Izel style. Key rules:
- 4-space indentation.
- Opening braces on same line.
- 100-column soft limit, 120-column hard limit.
- `draw` imports sorted alphabetically and grouped by ward.
- Trailing commas in all multi-line constructs.

### `izel-lint` — Linter

| Lint | Default | Description |
|------|---------|-------------|
| `unused_bindings` | warn | Declared but never used |
| `unused_effects` | warn | Function performs undeclared effects |
| `missing_contracts` | allow | No `@requires`/`@ensures` on public forge |
| `unchecked_witness` | warn | Witness in `raw` without SAFETY comment |
| `undocumented_unsafe` | error | `raw` block without SAFETY comment |
| `panic_in_pure` | error | `pure` function may panic |
| `implicit_cast` | deny | Implicit numeric widening |
| `large_stack_frame` | warn | Over 8KB of stack per function |
| `missing_docs` | allow | `open` declaration without `///` |

---

## 33. Project Directory Structure

```
izel/
├── Cargo.toml                      Rust workspace manifest
├── Izel.toml                       Future self-hosted build manifest
├── README.md
├── LICENSE
├── CHANGELOG.md
├── .github/
│   └── workflows/
│       ├── ci.yml                  Lint + test + fuzz on every PR
│       ├── nightly.yml             Extended tests + benchmarks
│       └── release.yml             Build and publish releases
│
├── crates/
│   ├── izel_span/                  Shared: source spans, byte offsets
│   ├── izel_session/               Shared: compilation config, CLI flags
│   ├── izel_diagnostics/           Shared: error types, rich terminal output
│   │
│   ├── izel_lexer/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── token.rs            Token kinds + spans
│   │   │   ├── lexer.rs            DFA lexer implementation
│   │   │   ├── number.rs           Numeric literal parsing
│   │   │   └── string.rs           String + interpolation lexing
│   │   ├── tests/
│   │   └── fuzz/                   Fuzz targets (cargo-fuzz)
│   │
│   ├── izel_parser/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── cst.rs              Lossless concrete syntax tree
│   │   │   ├── ast.rs              AST node definitions
│   │   │   ├── parser.rs           Recursive descent parser
│   │   │   ├── expr.rs             Pratt expression parser
│   │   │   ├── item.rs             Top-level item parser
│   │   │   └── error.rs            Parse error recovery
│   │   ├── tests/
│   │   └── fuzz/
│   │
│   ├── izel_ast_lower/             CST → desugared AST
│   ├── izel_resolve/               Name resolution + module graph
│   │
│   ├── izel_typeck/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── infer.rs            HM type inference + unification
│   │   │   ├── effects.rs          Effect set checking
│   │   │   ├── weave.rs            Weave resolution + coherence
│   │   │   ├── witness.rs          Witness type verification
│   │   │   ├── contracts.rs        @requires / @ensures checking
│   │   │   └── tast.rs             Typed AST definition
│   │   └── tests/
│   │
│   ├── izel_borrow/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ownership.rs
│   │   │   ├── region.rs           Lifetime regions (NLL)
│   │   │   ├── zone.rs             Memory zone escape analysis
│   │   │   └── error.rs            Borrow error messages
│   │   └── tests/
│   │
│   ├── izel_hir/                   HIR + monomorphizer + dual elaboration
│   ├── izel_mir/                   MIR (SSA + CFG) + drop + zone cleanup
│   │
│   ├── izel_opt/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pass.rs             Pass trait + pass manager
│   │   │   └── passes/
│   │   │       ├── const_fold.rs
│   │   │       ├── dce.rs
│   │   │       ├── inline.rs
│   │   │       ├── licm.rs
│   │   │       ├── tco.rs
│   │   │       ├── iter_fuse.rs    Pipeline fusion
│   │   │       ├── escape.rs
│   │   │       ├── sroa.rs
│   │   │       └── gvn.rs
│   │   └── tests/
│   │
│   ├── izel_codegen/               MIR => LLVM IR (inkwell)
│   ├── izel_driver/                Compiler entry point (izelc binary)
│   ├── izel_query/                 Incremental query system (salsa)
│   ├── izel_proc_macro/            Procedural macro support + built-in derives
│   │
│   ├── izel_std/                   Standard library (Izel source)
│   │   └── src/
│   │       ├── lib.iz
│   │       ├── prim/
│   │       ├── ops/
│   │       ├── iter/
│   │       ├── collections/
│   │       ├── io/
│   │       ├── fs/
│   │       ├── net/
│   │       ├── sync/
│   │       ├── async/
│   │       ├── alloc/
│   │       ├── math/
│   │       ├── hash/
│   │       ├── fmt/
│   │       ├── test/
│   │       └── bench/
│   │
│   ├── izel_lsp/                   LSP server
│   ├── izel_fmt/                   Formatter
│   ├── izel_lint/                  Linter
│   ├── izel_doc/                   Documentation generator
│   └── izel_pm/                    Package manager CLI (izel binary)
│
├── tests/
│   ├── compile_pass/               .iz files that must compile cleanly
│   ├── compile_fail/               .iz files with // ERROR: annotations
│   ├── run_pass/                   Programs with expected stdout/exit code
│   ├── run_fail/                   Programs expected to panic/fail
│   ├── effects/                    Effect system correctness tests
│   ├── witnesses/                  Witness type construction tests
│   ├── contracts/                  @requires/@ensures verification tests
│   ├── zones/                      Memory zone allocation and cleanup tests
│   └── snapshots/                  cargo-insta AST/HIR/MIR snapshot tests
│
├── docs/
│   ├── spec/                       Normative language specification
│   │   ├── 01-lexical.md
│   │   ├── 02-syntax.md
│   │   ├── 03-types.md
│   │   ├── 04-effects.md
│   │   ├── 05-memory.md
│   │   ├── 06-witnesses.md
│   │   ├── 07-contracts.md
│   │   ├── 08-zones.md
│   │   └── 09-duality.md
│   ├── reference/                  Standard library API reference
│   └── book/                       "The Izel Book" — full tutorial
│       ├── 00-intro.md
│       ├── 01-getting-started.md
│       ├── 02-ownership.md
│       ├── 03-types.md
│       ├── 04-effects.md
│       ├── 05-witnesses.md
│       ├── 06-zones.md
│       ├── 07-concurrency.md
│       └── 08-ffi.md
│
└── tools/
    ├── bootstrap/                  Bootstrap scripts (Rust -> Izel)
    ├── ci/                         CI helper scripts
    └── grammar/                    tree-sitter + ANTLR4 grammar files
```

---

## 34. Milestones & Roadmap

### Phase 0 — Bootstrap (Months 1–2)

- [ ] Define and stabilize the full token grammar
- [ ] Implement lexer with full test coverage and fuzz targets
- [ ] Minimal parser: variables, functions, arithmetic, `given`/`else`
- [ ] Minimal LLVM codegen: `main()`, arithmetic, `std::io::println`
- [ ] Compile and run "Hello, World!" in Izel

### Phase 1 — Core Language (Months 3–5)

- [ ] Full expression parser (Pratt, all operators)
- [ ] `shape`, `scroll`, `branch` (pattern matching)
- [ ] `weave` and `shape impl`
- [ ] Generics (monomorphized)
- [ ] Closures (`bind`) and higher-order functions
- [ ] Basic type inference
- [ ] `ward` module system and `draw` imports
- [ ] `izel new`, `izel build`, `izel run` CLI

### Phase 2 — Type System & Safety (Months 6–8)

- [ ] Complete HM type inference
- [ ] Weave coherence and orphan rule
- [ ] Effect system: declaration, checking, propagation
- [ ] Ownership and borrow checker (NLL)
- [ ] Lifetime inference and annotations
- [ ] `raw` blocks and `bridge` FFI
- [ ] High-quality error messages with code spans and suggestions

### Phase 3 — Unique Features (Months 9–12)

- [ ] Witness types (`NonZero`, `InBounds`, `Sorted`, custom)
- [ ] Temporal constraints (`@requires`, `@ensures`, `#[invariant]`)
- [ ] Memory zones (zone blocks, allocator, escape analysis)
- [ ] Cascade error system (`!` propagation with context chains)
- [ ] Duality types (`dual shape`) with round-trip verification
- [ ] `echo` compile-time blocks
- [ ] `|>` pipeline operator and iterator fusion optimizer pass

### Phase 4 — Standard Library v0.1 (Months 13–15)

- [ ] `std::prim`, `std::ops`, `std::cmp`, `std::iter`
- [ ] `std::option`, `std::result`, `std::fmt`
- [ ] `std::collections` (Vec, HashMap, BTreeMap)
- [ ] `std::io`, `std::fs`, `std::env`
- [ ] `std::thread`, `std::sync`, `std::atomic`
- [ ] `std::test` and `std::bench`
- [ ] `flow`/`tide` async runtime

### Phase 5 — Toolchain (Months 16–18)

- [ ] `izel-fmt`
- [ ] `izel-lsp` (completions + diagnostics)
- [ ] `izel-doc`
- [ ] `izel-lint`
- [ ] Full `izel` package manager with `Izel.toml`
- [ ] Cross-compilation support
- [ ] `#[derive(...)]` and procedural macro system

### Phase 6 — Optimization & Hardening (Months 19–22)

- [x] Full MIR optimizer (all passes)
- [x] SIMD intrinsics
- [x] Full `comptime` evaluation
- [x] Comprehensive snapshot + integration test suite
- [x] CI/CD with coverage + nightly fuzz runs
- [x] Language specification v1.0 (normative)

### Phase 7 — Self-Hosting (Months 23+)

- [x] Rewrite `izelc` in Izel
- [ ] Bootstrap: Rust-compiled Izel compiles Izel-written `izelc`
- [ ] Public Izel package registry
- [ ] `tree-sitter-izel` grammar for broad editor support
- [ ] Izel Playground (WASM-compiled browser REPL)

---

## 35. Dependencies

### Rust Crates

| Crate | Version | Purpose |
|-------|---------|---------|
| `inkwell` | 0.4+ | Safe LLVM bindings (code generation) |
| `llvm-sys` | 170+ | Low-level LLVM C API |
| `salsa` | 0.19+ | Incremental computation (LSP + queries) |
| `tower-lsp` | 0.20+ | LSP server framework |
| `codespan-reporting` | 0.11+ | Terminal diagnostics with source spans |
| `clap` | 4+ | CLI argument parsing |
| `toml` | 0.8+ | `Izel.toml` parsing |
| `serde` | 1+ | Serialization for IR / JSON output |
| `rayon` | 1+ | Parallel module compilation |
| `anyhow` | 1+ | Error handling in driver |
| `thiserror` | 1+ | Derive Error for compiler types |
| `tracing` | 0.1+ | Structured compiler logging |
| `tempfile` | 3+ | Temporary files during compilation |
| `semver` | 1+ | Package version parsing |
| `cargo-insta` | 1+ | Snapshot testing |
| `proptest` | 1+ | Property-based testing |
| `arbitrary` | 1+ | Structured fuzzing |
| `criterion` | 0.5+ | Micro-benchmarking |
| `winnow` | 0.6+ | Parser combinators for `Izel.toml` |
| `rustc-hash` | 1+ | Fast non-cryptographic hash |
| `indexmap` | 2+ | Ordered hash maps for symbol tables |
| `petgraph` | 0.6+ | Graph structures (module graph, CFG) |
| `string-interner` | 0.17+ | Interned identifiers |
| `smallvec` | 1+ | Stack-allocated small vectors |
| `bumpalo` | 3+ | Arena allocator for compiler internals |

### System Dependencies

| Dependency | Min Version | Notes |
|------------|-------------|-------|
| LLVM | 17.0 | Required; must match inkwell version |
| `lld` | 17.0 | Bundled or system linker |
| `clang` | 17.0 | Optional; C interop header parsing |
| CMake | 3.20 | Only if building LLVM from source |
| `zlib` | 1.2 | Required by LLVM |

---

## 36. Contributing Guidelines

### Getting Started

```bash
# macOS
brew install llvm@17
export LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17)

# Ubuntu / Debian
apt-get install llvm-17-dev clang-17 lld-17

# Clone and build
git clone https://github.com/VoxDroid/izel.git
cd izel
cargo build --workspace

# Full test suite
cargo nextest run --workspace

# Snapshot tests (review with: cargo insta review)
cargo insta test --workspace

# Fuzz the lexer
cargo fuzz run lexer_fuzz -- -max_total_time=60
```

### Commit Convention (Conventional Commits)

```
feat(typeck): add effect set unification for generic functions
fix(borrow): resolve false positive on mutable borrow across loop
perf(opt): implement pipeline fusion for map().filter() chains
test(witness): exhaustive tests for NonZero proof construction
docs(spec): document Cascade error chain semantics
refactor(mir): simplify CFG edge representation
```

### Pull Request Requirements

- All tests pass (`cargo nextest run --workspace`).
- No regressions in snapshot tests (`cargo insta test`).
- New language features require: unit tests, an integration test in `tests/`, and spec/book documentation.
- Breaking language changes require a proposal document in `docs/proposals/`.

### Testing Philosophy

| Test Type | Location | Purpose |
|-----------|----------|---------|
| Unit | `crates/*/tests/` | Individual function correctness |
| Snapshot | `tests/snapshots/` | AST/HIR/MIR stability |
| Compile-pass | `tests/compile_pass/` | Programs that must compile |
| Compile-fail | `tests/compile_fail/` | Programs that must fail with specific errors |
| Run-pass | `tests/run_pass/` | Programs with expected output |
| Effect tests | `tests/effects/` | Effect system correctness |
| Witness tests | `tests/witnesses/` | Witness construction and proofs |
| Contract tests | `tests/contracts/` | `@requires`/`@ensures` |
| Zone tests | `tests/zones/` | Zone allocation and cleanup |
| Fuzz | `crates/*/fuzz/` | Lexer/parser crash safety |
| Benchmarks | `crates/*/benches/` | Compiler throughput tracking |

---

## 37. Language Specification Index

The normative Izel language specification lives in `docs/spec/`:

| Chapter | File | Status |
|---------|------|--------|
| 1. Lexical Structure | `01-lexical.md` | Normative v1.0 |
| 2. Syntax & Grammar | `02-syntax.md` | Normative v1.0 |
| 3. Types & Kinds | `03-types.md` | Normative v1.0 |
| 4. The Effect System | `04-effects.md` | Normative v1.0 |
| 5. Memory & Ownership | `05-memory.md` | Normative v1.0 |
| 6. Witness Types | `06-witnesses.md` | Normative v1.0 |
| 7. Temporal Contracts | `07-contracts.md` | Normative v1.0 |
| 8. Memory Zones | `08-zones.md` | Normative v1.0 |
| 9. Duality Types | `09-duality.md` | Normative v1.0 |
| A. Grammar Reference | `appendix-a-grammar.md` | Normative v1.0 |
| B. Keyword Reference | `appendix-b-keywords.md` | Normative v1.0 |
| C. Standard Library API | `appendix-c-stdlib.md` | Normative v1.0 |

---

*This document is the living specification of the Izel programming language. It evolves with every design decision made. When in doubt, the rule is simple: does this make Izel more uniquely itself? If yes, it belongs here.*

---

**Izel** — *Only one.*
