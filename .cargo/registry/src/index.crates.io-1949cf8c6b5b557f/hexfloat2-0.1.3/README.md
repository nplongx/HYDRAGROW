
<!-- cargo-sync-readme start -->

`hexfloat2` supports hexadecimal f32/64 syntax.

IEEE754 defines a syntax for "hexadecimal-significand character sequences"
that lets you write a precise representation of a floating point number.

For example:
- `0x1.0p0` is just 1.0
- `0x8.8p1` is 8.5 * 2^1, or 17.
- `0x3.0p-12` is 3 * 2^-12, or 0.000732421875 in decimal.

Unlike decimal representations, "hexfloat" representations don't
involve any rounding, so a format-then-parse round trip will always
return exactly the same original value.

A formatted hexfloat will always appear in its "canonical" format,
copying the exact bit representation as closely as possible. For example,
the value 2^-20 will always be rendered as `0x1.0p-19`.

The parser attempts to handle "non-canonical" representations. For example,
these values will all be parsed as 2^-20:
- `0x1.0p-20`
- `0x2.0p-21`
- `0x0.0008p-7`

Some inputs won't parse: values with too
many hex digits (e.g. `0x10000000000000000p20`) will fail to parse
because the parser is only willing to parse up to 16 hex digits.
Values that are outside the range that can be represented in the
underlying type (f32 or f64) will also fail to parse.

Values with excessive precision will have the trailing bits dropped.
For example, `0x1.0000000000001p0` will be truncated to `1.0` when
parsed into a `HexFloat<f32>` (but would fit in an f64).

"Subnormal" values can be successfully formatted and parsed;
`0x0.000002p-127` can be parsed as an f32; anything smaller will
be truncated to 0.

# Examples
```rust
use hexfloat2::HexFloat32;

const EXPECTED: f32 = 1.0 / 1048576.0;

let x = "0x1.0p-20";
let fx: HexFloat32 = x.parse().unwrap();
assert_eq!(*fx, EXPECTED);

let y = "0x2.0p-21";
let fy: HexFloat32 = y.parse().unwrap();
assert_eq!(*fy, EXPECTED);

let z = "0x0.0008p-7";
let fz: HexFloat32 = z.parse().unwrap();
assert_eq!(*fz, EXPECTED);

let sz = format!("{fz}");
assert_eq!(sz, "0x1.000000p-20");
```

This crate provides newtype wrappers `HexFloat<T>`, aka `HexFloat32` or
`HexFloat64`, that implement `Display` and `FromStr`.

If you don't want to deal with the wrapper structs, you can also call
`hexfloat::parse::<T>()` or `hexfloat::format::<T>()` instead.


<!-- cargo-sync-readme end -->
