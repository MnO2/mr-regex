# mr-regex: Minimalist ASCII Regex-engine with 300 lines of Rust

[![Build Status](https://travis-ci.com/MnO2/mr-regex.svg?branch=master)](https://travis-ci.com/MnO2/mr-regex)

* This library compiles regex to NFA and then runs a DFS to search for the match
* It only supports ascii strings.
* Less than 300 lines of safe Rust.

# Examples

You can use a convience one line match function.

```rust
regex_match("(zz)+", "zz")
```

Or a more formal interface

```rust
let r = Regex::new("(zz)+".as_bytes()).unwrap();
r.is_match("zz".as_bytes())
```
