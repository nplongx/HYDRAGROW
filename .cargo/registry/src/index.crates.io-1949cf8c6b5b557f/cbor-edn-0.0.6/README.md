# cbor-edn ![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue) [![cbor-edn on crates.io](https://img.shields.io/crates/v/cbor-edn)](https://crates.io/crates/cbor-edn) [![cbor-edn on docs.rs](https://docs.rs/cbor-edn/badge.svg)](https://docs.rs/cbor-edn) [![Source Code Repository](https://img.shields.io/badge/Code-On%20Codeberg-blue?logo=Codeberg)](https://codeberg.org/chrysn/cbor-edn) ![Rust Version: 1.76.0](https://img.shields.io/badge/rustc-1.76.0-orange.svg)

## Tools for processing CBOR Diagnostic Notation (EDN)

The parser used by this crate is a PEG (Parsing Expression Grammer) parser built from the ABNF
used in the [EDN specification][__link0].

The crate’s main types represent not only the parsed items but also all the parts that have no
bearing on the translation to CBOR (spaces, commas, comments) and
choices that may or may not influence the CBOR (encoding indicators). This allows detailed
manipulation (for example inside comments) and a delayed processing of application oriented
literals.

Parsed values are expected to round-trip to identical representations when serialized. Most
manipulations of the values will ensure that their serialization output can also be
round-tripped from the internal format to the EDN serialization and back into the internal
format, but this can not be provided by all. (For example, removing all optional commas
while retaining comments would make the previous distinction between whether a comment was
before or after a comma indistinguishable).

Correct parsing does not guarantee that the value can also be encoded into CBOR. While there
are aspects that could be handled at parsing time and are not (eg. tag numbers exceeding the
encodable number space), there are cases that can not be handled by a library without further
context or privileges (eg. the e’’ application oriented literal that needs application context,
or the ref’’ application oriented literal that defers to relative files, accessing which can
involve file or network access). Consequentially, conversion to CBOR through the various
`.to_cbor()` methods is inherently fallible.

### Completeness

Known limitations are:

* Support for inspecting and constructing CBOR items is incomplete. The most common types can
  be constructed; contructing or inspecting more exotic items is possible through parsing
  hand-crafted EDN/CBOR and using the generated serializations, respectively.

* Options for attaching comments and space are limited and immature:
  
  * [`Item::with_comment()`][__link1] & [`StandaloneItem::set_comment`][__link2] can be used to add comments, but
    mainly produce [top-level items][__link3]. Deeper items are not configurable that
    way, as the comments don’t live in the item but its container.
  
  * Comments can be added to items through visitors such as [`Item::visit_map_elements`][__link4]; both
    the success and the error path of a visiting function can set comments around a tag.
  
  * Replacing an item with hand-crafted EDN (possibly from serialized item) is always an
    option.

* Indenting EDN works for the easy cases, but more exotic cases such as overflowing the limited
  width, long keys, or hash comments, easily disrupt the visual result.

### Security

This library does not access network or file system in any surprising ways and does not
endanger memory safety on its own. The main threat in using it is not resource bound: even
without packed CBOR, heavy nesting can easily overflow the stack, and the float conversions are
costly in time. Unless resource usage per user is limited, it is recommended to limit untrusted
user input to the length of repeated `{` characters that do not yet overflow the stack.

The crate has not been audited internally or externally. As the
[licenses][__link5]
[state][__link6], the software is provided “as is”.

### CLI application

Some functionality is available through a binary included with this crate:

<!-- See https://github.com/assert-rs/snapbox/issues/172 -->

```console
$ echo "[1, 2, 'x', ip'2001:db1::/64']" | cbor-edn diag2diag
[1, 2, 'x', ip'2001:db1::/64']
```


 [__cargo_doc2readme_dependencies_info]: ggGkYW0BYXSEG_W_Gn_kaocAGwCcVPfenh7eGy6gYLEwyIe4G6-xw_FwcbpjYXKEGxeidVyeRC7LG5Yg-dP30r5qG7imLBPRW6GeG7B4U05lepNHYWSBg2hjYm9yLWVkbmUwLjAuNmhjYm9yX2Vkbg
 [__link0]: https://www.ietf.org/archive/id/draft-ietf-cbor-edn-literals-15.html
 [__link1]: `Item::with_comment()`
 [__link2]: https://docs.rs/cbor-edn/0.0.6/cbor_edn/?search=StandaloneItem::set_comment
 [__link3]: https://docs.rs/cbor-edn/0.0.6/cbor_edn/struct.StandaloneItem.html
 [__link4]: https://docs.rs/cbor-edn/0.0.6/cbor_edn/?search=Item::visit_map_elements
 [__link5]: https://spdx.org/licenses/MIT.html
 [__link6]: https://spdx.org/licenses/Apache-2.0.html
