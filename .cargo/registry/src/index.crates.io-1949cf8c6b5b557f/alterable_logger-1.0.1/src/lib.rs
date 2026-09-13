// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Author: Simon Brummer (simon.brummer@posteo.de)

//! This module implements a thin wrapper for "log" allowing to replace the
//! actual logger implementation at run time.

// Imports
use std::sync::Arc;
use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

/// Configure/Re-configure logger
///
/// # Arguments
/// * filter: the filter level used for message filtering
/// * logger: a box containing the actual logger implementation use from now on.
pub fn configure(filter: log::LevelFilter, logger: Box<dyn log::Log>) {
    set_boxed_logger(logger);
    set_max_level(filter);
}

/// Set new logger
///
/// # Arguments
/// * logger: Logger replacing the currently registered logger.
pub fn set_boxed_logger(logger: Box<dyn log::Log>) {
    HANDLE.inner.swap(Arc::new(logger));
}

/// Set a new filter level
///
/// # Arguments
/// * filter: New Filter level used to filter log messages.
pub fn set_max_level(filter: log::LevelFilter) {
    log::set_max_level(filter);
}

// Implementation details
/// Handle containing an arc containing the current logger implementation.
static HANDLE: Lazy<Handle> = Lazy::new(|| {
    let handle = Handle {
        inner: Arc::new(ArcSwap::new(Arc::new(Box::new(NopLogger)))),
    };

    if let Err(_) = log::set_boxed_logger(Box::new(handle.clone())) {
        log::error!("Failed to set global logger (a logger was set previously).")
    };
    handle
});

/// Struct containing a handle to the current logger implementation in use.
///
/// Implements [log::Log] to fulfill the logging interface by forwarding
/// all calls to the current logger implementation.
#[derive(Clone)]
struct Handle {
    inner: Arc<ArcSwap<Box<dyn log::Log>>>,
}

impl log::Log for Handle {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.load().enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.inner.load().log(record)
    }

    fn flush(&self) {
        self.inner.load().flush()
    }
}

/// Dummy logger implementation used as initially registered logger.
struct NopLogger;

impl log::Log for NopLogger {
    fn enabled(&self, _: &log::Metadata) -> bool {
        false
    }

    fn log(&self, _: &log::Record) {}
    fn flush(&self) {}
}
