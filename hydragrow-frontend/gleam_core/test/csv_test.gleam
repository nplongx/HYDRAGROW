import csv
import gleeunit/should

pub fn escape_field_test() {
  // Bọc ngoặc kép và nhân đôi ngoặc kép bên trong
  csv.escape_field("Hello \"World\"")
  |> should.equal("\"Hello \"\"World\"\"\"")
}

pub fn build_row_test() {
  csv.build_row(["Device01", "Monitoring", "OK"])
  |> should.equal("\"Device01\",\"Monitoring\",\"OK\"")
}
