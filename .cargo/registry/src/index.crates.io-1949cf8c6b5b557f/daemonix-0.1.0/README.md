# dameonix

This is a maintained fork of the [daemonize] crate, which in turn was inspired
by the [thesharp/daemonize] Python library.

dameonix v0.1.0 is identical with daemonize v0.5.0 but with a fix for the
reported [UB issue] applied. Future major versions will likely change or remove
APIs to reduce the amount of `unsafe` code that needs to be audited and
documented.

### Usage example:

```rust
use std::fs::File;

use daemonix::Daemonize;

fn main() {
    let stdout = File::create("/tmp/daemon.out").unwrap();
    let stderr = File::create("/tmp/daemon.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/test.pid") // Every method except `new` and `start`
        .chown_pid_file(true)      // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        .user("nobody")
        .group("daemon") // Group name
        .group(2)        // or group id.
        .umask(0o777)    // Set umask, `0o027` by default.
        .stdout(stdout)  // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr)  // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }
}
```

### License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/license/mit>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms or
conditions.

[daemonize]: https://crates.io/crates/daemonize
[thesharp/daemonize]: https://github.com/thesharp/daemonize
[UB issue]: https://github.com/knsd/daemonize/pull/57
