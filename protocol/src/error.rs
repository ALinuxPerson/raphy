use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Deserialize, Serialize, Encode, Decode)]
pub struct SerdeError {
    display: String,
    alt_display: String,
    debug: String,
    alt_debug: String,
    source: Option<Box<Self>>,
}

impl SerdeError {
    pub fn new<T>(e: &T) -> Self
    where
        T: ?Sized + std::error::Error,
    {
        Self {
            display: e.to_string(),
            alt_display: format!("{e:#}"),
            debug: format!("{e:?}"),
            alt_debug: format!("{e:#?}"),
            source: e.source().map(|s| Box::new(Self::new(s))),
        }
    }
}

impl std::error::Error for SerdeError {
    fn source(&self) -> Option<&(dyn 'static + std::error::Error)> {
        self.source
            .as_ref()
            .map(|s| &**s as &(dyn 'static + std::error::Error))
    }

    fn description(&self) -> &str {
        &self.display
    }
}

impl fmt::Display for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:#}", self.display)
        } else {
            write!(f, "{}", self.display)
        }
    }
}

impl fmt::Debug for SerdeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:#?}", self.display)
        } else {
            write!(f, "{:?}", self.display)
        }
    }
}
