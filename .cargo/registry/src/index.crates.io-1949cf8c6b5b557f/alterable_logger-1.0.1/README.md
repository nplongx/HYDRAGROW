# alterable_logger

This crate implements a thin wrapper for the widely used "log" crate adding the functionality
to replace the logger implementation in use at run-time. This allows more variety in terms of
logger usage at a small performance penalty.

The run-time re-configuration implementation is based on Arc, ArcSwap and Box, therefore
this crate supports only std environments.

# Usage (excerpt from examples/usage/src/main.rs)

```
use log::LevelFilter;
use simplelog::{SimpleLogger, Config};

fn main() {
    log::error!("You should not see this!");

    // Register a basic logger with maximum filter level warn
    let logger = SimpleLogger::new(LevelFilter::Warn, Config::default());
    alterable_logger::configure(LevelFilter::Error, logger);

    log::error!("You should see this!");
    log::warn!("But this is invisible!");

    // Change the log level to warning
    alterable_logger::set_max_level(LevelFilter::Warn);
    log::warn!("You should see this!");

    // Change the log level to info (this will be not visible as the logger does not support it)
    alterable_logger::set_max_level(LevelFilter::Info);
    log::info!("But this is invisible!");

    // Replace current logger with a logger that supports a more verbose logging
    let logger = SimpleLogger::new(LevelFilter::Info, Config::default());
    alterable_logger::set_boxed_logger(logger);
    log::info!("You should see this!");
}
```

# Hints

Although alterable_logger supports changing the logging implementation, it
needs to be the first registered logger of an application. Otherwise the logger registration
will fail silently and the first registered logger stays the logger in use.

# Performance

The logging facilities are registered globally, this makes it difficult to profile the
"log" facilities against "alterable_logger" in a single application. To get a rough estimation
on the performance penalty introduced by alterable_logger, two applications are shipped under examples.

Here are the results:

```
Log profiling test program
--------------------------
Log info messages 1000000 times
Total operation time: 265876362ns
Operation time per cycle: 265ns
```

```
Alterable_logger profiling test program
---------------------------------------
Log info messages 1000000 times
Total operation time: 647119571ns
Operation time per cycle: 647ns
```

Compared to the default logger, the indirection and locking of the alterable_logger
increases the operation time by more than factor 2. In real world applications this should not
matter much, because most log implementations perform IO intensive operations that introduce way more overhead.

