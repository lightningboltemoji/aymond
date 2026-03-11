use aws_sdk_dynamodb::config::http::HttpResponse;
use aws_sdk_dynamodb::error::SdkError;
use std::fmt;

#[derive(Debug)]
pub enum BatchError<E: fmt::Debug + std::error::Error + 'static> {
    SdkError(Box<SdkError<E, HttpResponse>>),
    RetriesExhausted { message: String },
}

impl<E: fmt::Debug + std::error::Error + 'static> fmt::Display for BatchError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchError::SdkError(e) => write!(f, "{e}"),
            BatchError::RetriesExhausted { message } => write!(f, "{message}"),
        }
    }
}

impl<E: fmt::Debug + std::error::Error + 'static> std::error::Error for BatchError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BatchError::SdkError(e) => Some(e.as_ref()),
            BatchError::RetriesExhausted { .. } => None,
        }
    }
}

impl<E: fmt::Debug + std::error::Error + 'static> From<SdkError<E, HttpResponse>>
    for BatchError<E>
{
    fn from(err: SdkError<E, HttpResponse>) -> Self {
        BatchError::SdkError(Box::new(err))
    }
}
