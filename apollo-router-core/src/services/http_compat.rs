//! Wrapper types for [`http::Request`] and [`http::Response`] from the http crate.
//!
//! To improve their usability.

use std::{
    cmp::PartialEq,
    hash::Hash,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "axum-server")]
use axum::{body::boxed, response::IntoResponse};
#[cfg(feature = "axum-server")]
use bytes::Bytes;

#[cfg(feature = "axum-server")]
use crate::ResponseBody;

use http::{header::HeaderName, request::Parts, uri::InvalidUri, HeaderValue, Method, Uri};

#[derive(Debug)]
pub struct Request<T> {
    inner: http::Request<T>,
}

// Most of the required functionality is provided by our Deref and DerefMut implementations.
#[buildstructor::builder]
impl<T> Request<T> {
    /// This is the constructor (or builder) to use when constructing a real Request.
    ///
    /// Required parameters are required in non-testing code to create a Request.
    pub fn new(
        headers: Vec<(HeaderName, HeaderValue)>,
        uri: http::Uri,
        method: http::Method,
        body: T,
    ) -> http::Result<Request<T>> {
        let mut builder = http::request::Builder::new().method(method).uri(uri);
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        let req = builder.body(body)?;

        Ok(Self { inner: req })
    }

    /// This is the constructor (or builder) to use when constructing a "fake" Request.
    ///
    /// This does not enforce the provision of the uri and method that is required for a fully functional
    /// Request. It's usually enough for testing, when a fully consructed Request is
    /// difficult to construct and not required for the purposes of the test.
    pub fn fake_new(
        headers: Vec<(HeaderName, HeaderValue)>,
        uri: Option<http::Uri>,
        method: Option<http::Method>,
        body: T,
    ) -> http::Result<Request<T>> {
        let mut builder = http::request::Builder::new()
            .method(method.unwrap_or(Method::GET))
            .uri(uri.unwrap_or_else(|| Uri::from_static("http://test")));
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
        let req = builder.body(body)?;

        Ok(Self { inner: req })
    }

    /// Update the associated URL
    pub fn from_parts(head: Parts, body: T) -> Request<T> {
        Request {
            inner: http::Request::from_parts(head, body),
        }
    }

    /// Consumes the request, returning just the body.
    pub fn into_body(self) -> T {
        self.inner.into_body()
    }

    /// Consumes the request returning the head and body parts.
    pub fn into_parts(self) -> (http::request::Parts, T) {
        self.inner.into_parts()
    }

    /// Consumes the request returning a new request with body mapped to the return type of the passed in function.
    pub fn map<F, U>(self, f: F) -> Result<Request<U>, InvalidUri>
    where
        F: FnOnce(T) -> U,
    {
        Ok(Request {
            inner: self.inner.map(f),
        })
    }
}

impl<T> Request<T>
where
    T: Default,
{
    // Only used for plugin::utils and tests
    pub fn mock() -> Request<T> {
        Request {
            inner: http::Request::default(),
        }
    }
}

impl<T> Deref for Request<T> {
    type Target = http::Request<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Request<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Clone> Clone for Request<T> {
    fn clone(&self) -> Self {
        // note: we cannot clone the extensions because we cannot know
        // which types were stored
        let mut req = http::Request::builder()
            .method(self.inner.method().clone())
            .version(self.inner.version())
            .uri(self.inner.uri().clone());
        req.headers_mut().unwrap().extend(
            self.inner
                .headers()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );

        let req = req
            .body(self.inner.body().clone())
            .expect("cloning a valid request creates a valid request");
        Self { inner: req }
    }
}

impl<T: Hash> Hash for Request<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.method().hash(state);
        self.inner.version().hash(state);
        self.inner.uri().hash(state);
        // this assumes headers are in the same order
        for (name, value) in self.inner.headers() {
            name.hash(state);
            value.hash(state);
        }
        self.inner.body().hash(state);
    }
}

impl<T: PartialEq> PartialEq for Request<T> {
    fn eq(&self, other: &Self) -> bool {
        let mut res = self.inner.method().eq(other.inner.method())
            && self.inner.version().eq(&other.inner.version())
            && self.inner.uri().eq(other.inner.uri());

        if !res {
            return false;
        }
        if self.inner.headers().len() != other.inner.headers().len() {
            return false;
        }

        // this assumes headers are in the same order
        for ((name, value), (other_name, other_value)) in self
            .inner
            .headers()
            .iter()
            .zip(other.inner.headers().iter())
        {
            res = name.eq(other_name) && value.eq(other_value);
            if !res {
                return false;
            }
        }

        self.inner.body().eq(other.inner.body())
    }
}

impl<T: PartialEq> Eq for Request<T> {}

#[derive(Debug, Default)]
pub struct Response<T> {
    pub inner: http::Response<T>,
}

impl<T> Response<T> {
    pub fn into_parts(self) -> (http::response::Parts, T) {
        self.inner.into_parts()
    }

    pub fn into_body(self) -> T {
        self.inner.into_body()
    }

    pub fn map<F, U>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        self.inner.map(f).into()
    }
}

impl<T> Deref for Response<T> {
    type Target = http::Response<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Response<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<http::Response<T>> for Response<T> {
    fn from(inner: http::Response<T>) -> Self {
        Response { inner }
    }
}

impl<T> From<Response<T>> for http::Response<T> {
    fn from(response: Response<T>) -> http::Response<T> {
        response.inner
    }
}

impl<T: Clone> Clone for Response<T> {
    fn clone(&self) -> Self {
        // note: we cannot clone the extensions because we cannot know
        // which types were stored
        let mut res = http::Response::builder()
            .status(self.inner.status())
            .version(self.inner.version());
        res.headers_mut().unwrap().extend(
            self.inner
                .headers()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone())),
        );

        let res = res
            .body(self.inner.body().clone())
            .expect("cloning a valid response creates a valid response");
        Self { inner: res }
    }
}

#[cfg(feature = "axum-server")]
impl IntoResponse for Response<ResponseBody> {
    fn into_response(self) -> axum::response::Response {
        // todo: chunks?
        let (parts, body) = self.into_parts();
        let json_body_bytes =
            Bytes::from(serde_json::to_vec(&body).expect("body should be serializable; qed"));

        axum::response::Response::from_parts(parts, boxed(http_body::Full::new(json_body_bytes)))
    }
}

#[cfg(feature = "axum-server")]
impl IntoResponse for Response<Bytes> {
    fn into_response(self) -> axum::response::Response {
        // todo: chunks?
        let (parts, body) = self.into_parts();

        axum::response::Response::from_parts(parts, boxed(http_body::Full::new(body)))
    }
}
