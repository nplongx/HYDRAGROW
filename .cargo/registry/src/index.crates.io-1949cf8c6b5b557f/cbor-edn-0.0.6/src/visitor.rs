//! Visitor building blocks.
//!
//! A [`Visitor`] is an object that is called, in sequence, with many components of an CBOR item or
//! sequence tree.
//!
//! This makes sense not only for the mere purpose of abstraction (things worked fine and
//! maintainably in earlier versions), but also serves to enable influence on the environment:
//! Processing something (eg. an EDN [`Item`][crate::Item]) may result in an effect on the container (eg. when
//! processing an item fails, and the error should be shown in a comment next to it -- but comments
//! are part of the space between items, and need to be applied there by the container; made harder
//! in Rust where during iteration, both the previous and the next item might want to write into
//! the same space).
//!
//! Right now, the visitor only gets to process items. Other things (such as space or
//! NonemptyMscVec lists) could be made processable when needed.

pub(crate) trait Visitor {
    /// This function gets called for every item by the various visitors.
    ///
    /// If the action can not be performed on the item in a self-contained way, it may bubble up a
    /// non-default [`ProcessResult`].
    fn process(&mut self, _item: &mut crate::Item<'_>) -> ProcessResult {
        ProcessResult::default()
    }
}

/// Description of actions to take around the item.
///
/// This describes only actions that can not be done by acting on a [`&mut crate::Item`] itself;
/// for example, wrapping it in a tag or replacing it with an application oriented literal is not a
/// [`ProcessResult`], but an action taken in [`Visitor::process`].
#[must_use]
#[derive(PartialEq)]
pub(crate) struct ProcessResult {
    comments_after: Vec<String>,
    recurse: bool,
}

impl Default for ProcessResult {
    fn default() -> Self {
        Self {
            comments_after: vec![],
            recurse: true,
        }
    }
}

/// # Building a `ProcessResult`
impl ProcessResult {
    pub(crate) fn with_comment_after(mut self, comment: impl Into<String>) -> Self {
        self.comments_after.push(comment.into());
        self
    }

    /// Configure the visitation to not recurse into this item.
    ///
    /// This might be a bit atypical of visitors, and one might think that direct access to a top
    /// level item's members (array elements or map key-value pairs) would work better, but those
    /// accessors can not act on the iteration item's surroundings, while a visitor can.
    pub(crate) fn stop_recursion(self) -> Self {
        Self {
            recurse: false,
            ..self
        }
    }

    pub(crate) fn chain(self, other: Self) -> Self {
        Self {
            recurse: self.recurse && other.recurse,
            comments_after: self
                .comments_after
                .into_iter()
                .chain(other.comments_after)
                .collect(),
        }
    }
}

/// # Consuming a `ProcessResult`
///
/// These methods are called by recipients: they work off all the actionables by calling the
/// `.to_…` functions with suitable recipients of the actions, ending with [`.done()`][Self::done].
impl ProcessResult {
    // If we typestated a bit more, we could avoid this (if use_space_before ->
    // ProcessResultHalfProcessed)
    #[track_caller]
    pub(crate) fn done(self) {
        if self != Default::default() {
            panic!(
                "use_space_before() and use_space_after() have been called, this should be empty."
            );
        }
    }

    pub(crate) fn take_recurse(&mut self) -> bool {
        let recurse = self.recurse;
        // FIXME: It'd be prettier if we could reliably detect this and not just when it deviates
        // from the default, but that'd need explicit setting in a process result.
        self.recurse = true;
        recurse
    }

    pub(crate) fn use_space_before(self, _s: &mut impl crate::space::Spaceish) -> Self {
        self
    }

    pub(crate) fn use_space_after(mut self, s: &mut impl crate::space::Spaceish) -> Self {
        for comment in self.comments_after.drain(..) {
            s.prepend_comment(&comment);
        }
        self
    }
}

pub(crate) struct ApplicationLiteralsVisitor<F> {
    pub(crate) user_fn: F,
}

impl<F: for<'b> FnMut(String, String, &mut crate::Item<'b>) -> Result<(), String>> Visitor
    for ApplicationLiteralsVisitor<F>
{
    fn process(&mut self, item: &mut crate::Item<'_>) -> ProcessResult {
        let Ok((identifier, value)) = item.get_application_literal() else {
            return ProcessResult::default();
        };
        match (self.user_fn)(identifier, value, item) {
            Ok(()) => ProcessResult::default(),
            Err(e) => ProcessResult::default().with_comment_after(e),
        }
    }
}

pub(crate) struct TagVisitor<F> {
    pub(crate) user_fn: F,
}

impl<F: for<'b> FnMut(u64, &mut crate::Item<'b>) -> Result<(), String>> Visitor for TagVisitor<F> {
    fn process(&mut self, item: &mut crate::Item<'_>) -> ProcessResult {
        let Ok(tag) = item.get_tag() else {
            return ProcessResult::default();
        };
        match (self.user_fn)(tag, item) {
            Ok(()) => ProcessResult::default(),
            Err(e) => ProcessResult::default().with_comment_after(e),
        }
    }
}

pub(crate) struct ArrayElementVisitor<F> {
    seen_first: bool,
    user_fn: F,
}

impl<F> ArrayElementVisitor<F> {
    pub(crate) fn new(user_fn: F) -> Self {
        Self {
            seen_first: false,
            user_fn,
        }
    }
}

impl<F: for<'b> FnMut(&mut crate::Item<'b>) -> Result<Option<String>, String>> Visitor
    for ArrayElementVisitor<F>
{
    fn process(&mut self, item: &mut crate::Item<'_>) -> ProcessResult {
        if !self.seen_first {
            // We don't really iterate, just checking if it is an array
            if item.get_array_items_mut().is_err() {
                return ProcessResult::default()
                    .with_comment_after("Expected array".to_string())
                    .stop_recursion();
            }
            self.seen_first = true;
            return ProcessResult::default();
        }

        match (self.user_fn)(item) {
            // We currently don't distinguish between comment and error in the output (but may
            // later)
            Ok(Some(text)) => ProcessResult::default().with_comment_after(text),
            Ok(None) => ProcessResult::default(),
            Err(e) => ProcessResult::default().with_comment_after(e),
        }
        .stop_recursion()
    }
}

pub(crate) struct MapElementVisitor<F> {
    seen_first: bool,
    /// Handler to be applied to the next map value.
    ///
    /// Visiting a key sets this to `Some(_)` (containing whatever the user_fn gave for the key,
    /// which may be `None`), visiting the next item (which is the value) takes that and leaves
    /// `None`.
    next_value_handler: Option<Option<MapValueHandler>>,
    user_fn: F,
}

impl<F> MapElementVisitor<F> {
    pub(crate) fn new(user_fn: F) -> Self {
        Self {
            seen_first: false,
            next_value_handler: None,
            user_fn,
        }
    }
}

pub type MapValueHandler =
    Box<dyn for<'data> FnOnce(&mut crate::Item<'data>) -> Result<Option<String>, String>>;

impl<
        F: for<'b> FnMut(
            &mut crate::Item<'b>,
        ) -> Result<(Option<String>, Option<MapValueHandler>), String>,
    > Visitor for MapElementVisitor<F>
{
    fn process(&mut self, item: &mut crate::Item<'_>) -> ProcessResult {
        if !self.seen_first {
            // We don't really iterate, just checking if it is an array
            if item.get_map_items().is_err() {
                return ProcessResult::default()
                    .with_comment_after("Expected map".to_string())
                    .stop_recursion();
            }
            self.seen_first = true;
            return ProcessResult::default();
        }

        if let Some(handler) = self.next_value_handler.take() {
            if let Some(handler) = handler {
                match (handler)(item) {
                    // We currently don't distinguish between comment and error in the output (but may
                    // later)
                    Ok(Some(text)) => ProcessResult::default().with_comment_after(text),
                    Ok(None) => ProcessResult::default(),
                    Err(e) => ProcessResult::default().with_comment_after(e),
                }
            } else {
                Default::default()
            }
        } else {
            match (self.user_fn)(item) {
                // We currently don't distinguish between comment and error in the output (but may
                // later)
                Ok((comment, value_handler)) => {
                    self.next_value_handler = Some(value_handler);
                    match comment {
                        Some(text) => ProcessResult::default().with_comment_after(text),
                        None => ProcessResult::default(),
                    }
                }
                Err(e) => ProcessResult::default().with_comment_after(e),
            }
        }
        .stop_recursion()
    }
}
