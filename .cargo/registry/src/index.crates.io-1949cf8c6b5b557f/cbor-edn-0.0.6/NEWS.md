# Changes in 0.0.6

* Updated to draft version -16.
* Major API changes due to -14 moving away from `S item S` idiom:
  * Split StandaloneItem and Item.
  * Modification is now done more with visitor callbacks that can leave comments to be applied in their result.
  * Several "whitespace" modifiers renamed to "delimiter" modifiers as they now affect commas too.
* Changes in produced delimites for better and worse; sequences now come without trailing comma.

# Changes in 0.0.5

* MSRV decreased to 1.76.
* Various clippy fixes.
* CI and test adjustments.
