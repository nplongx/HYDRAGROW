def fix_file(path, replacements):
    with open(path, "r") as f:
        content = f.read()
    for search, replace in replacements:
        content = content.replace(search, replace)
    with open(path, "w") as f:
        f.write(content)

fix_file("hydragrow-simulator/tests/mqtt_integration.rs", [
    ("""if let Ok(n) = pub_conn.read(&mut buf) {
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }""",
    """if let Ok(n) = pub_conn.read(&mut buf) {
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }"""),
])

# Let's write the file again for mqtt_integration.rs to fix collapsible if.
import re

with open("hydragrow-simulator/tests/mqtt_integration.rs", "r") as f:
    content = f.read()

content = content.replace("""if let Ok(n) = pub_conn.read(&mut buf) {
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }""", """if let Ok(n) = pub_conn.read(&mut buf) {
                    if n > 0 {
                        let _ = sub_conn.write_all(&buf[..n]);
                        let _ = sub_conn.flush();
                    }
                }""")
