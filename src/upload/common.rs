use std::{
    fmt::{Debug, Display},
    thread::JoinHandle,
};

use reqwest::{blocking::Response, Error, StatusCode};
use serde::Deserialize;

#[derive(Debug)]
pub struct RequestError {
    action_error: &'static str,
    kind: RequestErrorKind,
}

#[derive(Debug)]
pub enum RequestErrorKind {
    Status(StatusCode, Option<String>),
    Err(Error),
    File(std::io::Error),
}

impl RequestError {
    fn new(kind: RequestErrorKind) -> Self {
        Self {
            action_error: "Action failed.",
            kind,
        }
    }

    pub fn action_error(mut self, error: &'static str) -> Self {
        self.action_error = error;
        self
    }

    fn fmt_base(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.action_error)
    }

    fn fmt_status_code(f: &mut std::fmt::Formatter<'_>, status: StatusCode) -> std::fmt::Result {
        write!(f, "\n\nStatus Code: {}", status)
    }

    fn fmt_details_or_status_and_error(
        f: &mut std::fmt::Formatter<'_>,
        status: StatusCode,
        error: &Option<String>,
    ) -> std::fmt::Result {
        match error
            .as_ref()
            .map(|e| serde_json::from_str::<ServerError>(e).ok())
            .flatten()
        {
            Some(error) => write!(f, "\n\nDetails: {}", error.detail)?,
            None => {
                Self::fmt_status_code(f, status)?;
                Self::fmt_error(f, error)?;
            }
        }

        Ok(())
    }

    fn fmt_error(f: &mut std::fmt::Formatter<'_>, error: &Option<String>) -> std::fmt::Result {
        if let Some(error) = error.as_ref() {
            write!(f, "\n\nError: {}", error)?;
        }

        Ok(())
    }
}

impl From<Error> for RequestError {
    fn from(value: Error) -> Self {
        Self::new(RequestErrorKind::Err(value))
    }
}

impl From<Response> for RequestError {
    fn from(value: Response) -> Self {
        Self::new(RequestErrorKind::Status(value.status(), value.text().ok()))
    }
}

impl From<std::io::Error> for RequestError {
    fn from(value: std::io::Error) -> Self {
        Self::new(RequestErrorKind::File(value))
    }
}

impl Display for RequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_base(f)?;
        match &self.kind {
            RequestErrorKind::Status(status, error) => {
                if *status == StatusCode::INTERNAL_SERVER_ERROR {
                    Self::fmt_details_or_status_and_error(f, *status, error)?;
                } else {
                    Self::fmt_status_code(f, *status)?;
                    Self::fmt_error(f, error)?;
                }
            }
            RequestErrorKind::Err(err) => {
                write!(f, "Failed to upload combat log.\n\nError: {}", err)?
            }
            RequestErrorKind::File(err) => write!(f, "{}", err)?,
        }

        Ok(())
    }
}

#[derive(Deserialize)]
pub struct ServerError {
    detail: String,
}

pub fn spawn_request<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(f)
        .unwrap()
}
