import * as $should from "../gleeunit/gleeunit/should.mjs";
import * as $csv from "./csv.mjs";
import { toList } from "./gleam.mjs";

export function escape_field_test() {
  let _pipe = $csv.escape_field("Hello \"World\"");
  return $should.equal(_pipe, "\"Hello \"\"World\"\"\"");
}

export function build_row_test() {
  let _pipe = $csv.build_row(toList(["Device01", "Monitoring", "OK"]));
  return $should.equal(_pipe, "\"Device01\",\"Monitoring\",\"OK\"");
}
