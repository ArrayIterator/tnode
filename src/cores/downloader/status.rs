use std::fmt::Display;
use std::sync::Arc;
use crate::cores::system::error::Error;

#[derive(Debug, Clone)]
pub enum Status {
    Pending,
    Completed,
    InProgress(String), // Include a message for progress updates
    Cancelled(String), // Include a reason for cancellation
    Failed(Arc<Error>),     // Include an error message for failed downloads
}

impl Status {
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
    pub fn is_in_progress(&self) -> bool {
        matches!(self, Self::InProgress(_))
    }
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled(_))
    }
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
    pub fn is_finished(&self) -> bool {
        !self.is_in_progress() && !self.is_pending()
    }
    pub fn is_error(&self) -> bool {
        self.is_failed() || self.is_cancelled()
    }
    pub fn get_progress_message(&self) -> Option<&str> {
        if let Self::InProgress(message) = self {
            Some(message)
        } else {
            None
        }
    }
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cancelled(l0), Self::Cancelled(r0)) => l0 == r0,
            (Self::Failed(l0), Self::Failed(r0)) => {
                if l0.error_type != r0.error_type || l0.message != r0.message {
                    return false;
                }
                let s = l0.original_error.as_ref().map(|e| e.to_string());
                let s1 = r0.original_error.as_ref().map(|e| e.to_string());
                s == s1
            }
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Pending => write!(f, "Pending"),
            Status::InProgress(e) => write!(f, "Progress : {}", e),
            Status::Completed => write!(f, "Completed"),
            Status::Cancelled(reason) => write!(f, "Cancelled: {}", reason),
            Status::Failed(error) => write!(f, "Failed: {}", error),
        }
    }
}
