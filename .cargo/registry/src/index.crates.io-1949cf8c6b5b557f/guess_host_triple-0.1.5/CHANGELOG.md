All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog]
and this project adheres to [Semantic Versioning].

[Keep a Changelog]: https://keepachangelog.com/en/1.0.0/
[Semantic Versioning]: https://semver.org/spec/v2.0.0.html

[Unreleased]
============

Nothing.

[0.1.5] - 2025-06-30
====================

Fixed
-----

  - Understand `amd64` as a synonym for `x86_64`

[0.1.4] - 2024-08-24
====================

Added
-----

  - We now detect `aarch64-pc-windows-msvc` for Windows on Aarch64.

[0.1.3] - 2021-09-22
====================

Added
-----

  - We now detect `aarch64-apple-darwin` as macOS on M1

[0.1.2] - 2020-04-01
====================

Changed
-------

  - Internal safety polish
      - Made `unsafe` blocks as short as possible,
        and added safety rationales for each one.
      - Replace deprecated `std::mem::zeroed()`
        with `std::mem::MaybeUninit`
      - Use `CStr` and `CString`
        instead of manually handling `c_char` arrays

[0.1.1] - 2020-03-26
====================

Added
-----

  - Started keeping a change log.

Changed
-------

  - Use the `errno` crate to access the C standard library's error codes,
    not the `libc` crate.
    [Apparently](https://github.com/rust-lang/rfcs/pull/1571),
    the `libc` crate is intended to expose C types and functions to Rust,
    not weird quasi-variable things like `errno`.

[0.1.0] - 2018-04-24
====================

Initial release.

[Unreleased]: https://gitlab.com/Screwtapello/guess_host_triple/compare/v0.1.5...master
[0.1.5]: https://gitlab.com/Screwtapello/guess_host_triple/compare/v0.1.4...v0.1.5
[0.1.4]: https://gitlab.com/Screwtapello/guess_host_triple/compare/v0.1.3...v0.1.4
[0.1.3]: https://gitlab.com/Screwtapello/guess_host_triple/compare/v0.1.2...v0.1.3
[0.1.2]: https://gitlab.com/Screwtapello/guess_host_triple/compare/v0.1.1...v0.1.2
[0.1.1]: https://gitlab.com/Screwtapello/guess_host_triple/compare/v0.1.0...v0.1.1
[0.1.0]: https://gitlab.com/Screwtapello/guess_host_triple/tree/v0.1.0
