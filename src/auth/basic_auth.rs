use base64::Engine;

use crate::headers::{HeaderName, HeaderValue, Headers, AUTHORIZATION};
use crate::Status;
use crate::{
    auth::{AuthenticationScheme, Authorization},
    headers::Header,
};
use crate::{bail_status as bail, ensure_status as ensure};

/// HTTP Basic authorization.
///
/// # Specifications
///
/// - [RFC7617](https://tools.ietf.org/html/rfc7617)
///
/// # Examples
///
/// ```
/// # fn main() -> http_types::Result<()> {
/// #
/// use http_types::Response;
/// use http_types::auth::{AuthenticationScheme, BasicAuth};
///
/// let username = "nori";
/// let password = "secret_fish!!";
/// let authz = BasicAuth::new(username, password);
///
/// let mut res = Response::new(200);
/// res.insert_header(&authz, &authz);
///
/// let authz = BasicAuth::from_headers(res)?.unwrap();
///
/// assert_eq!(authz.username(), username);
/// assert_eq!(authz.password(), password);
/// #
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct BasicAuth {
    username: String,
    password: String,
}

impl BasicAuth {
    /// Create a new instance of `BasicAuth`.
    pub fn new<U, P>(username: U, password: P) -> Self
    where
        U: AsRef<str>,
        P: AsRef<str>,
    {
        let username = username.as_ref().to_owned();
        let password = password.as_ref().to_owned();
        Self { username, password }
    }

    /// Create a new instance from headers.
    pub fn from_headers(headers: impl AsRef<Headers>) -> crate::Result<Option<Self>> {
        let auth = match Authorization::from_headers(headers)? {
            Some(auth) => auth,
            None => return Ok(None),
        };

        let scheme = auth.scheme();
        ensure!(
            matches!(scheme, AuthenticationScheme::Basic),
            400,
            "Expected basic auth scheme found `{}`",
            scheme
        );
        Self::from_credentials(auth.credentials()).map(Some)
    }

    /// Create a new instance from the base64 encoded credentials.
    pub fn from_credentials(credentials: impl AsRef<[u8]>) -> crate::Result<Self> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(credentials)
            .status(400)?;
        let credentials = String::from_utf8(bytes).status(400)?;

        let mut iter = credentials.splitn(2, ':');
        let username = iter.next();
        let password = iter.next();

        let (username, password) = match (username, password) {
            (Some(username), Some(password)) => (username.to_string(), password.to_string()),
            (Some(_), None) => bail!(400, "Expected basic auth to contain a password"),
            (None, _) => bail!(400, "Expected basic auth to contain a username"),
        };

        Ok(Self { username, password })
    }

    /// Get the username.
    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    /// Get the password.
    pub fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl Header for BasicAuth {
    fn header_name(&self) -> HeaderName {
        AUTHORIZATION
    }

    fn header_value(&self) -> HeaderValue {
        let scheme = AuthenticationScheme::Basic;

        let mut credentials = String::new();
        base64::engine::general_purpose::STANDARD.encode_string(
            format!("{}:{}", self.username, self.password),
            &mut credentials,
        );

        let auth = Authorization::new(scheme, credentials);
        auth.header_value()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::headers::Headers;

    #[test]
    fn smoke() -> crate::Result<()> {
        let username = "nori";
        let password = "secret_fish!!";
        let authz = BasicAuth::new(username, password);

        let mut headers = Headers::new();
        authz.apply_header(&mut headers);

        let authz = BasicAuth::from_headers(headers)?.unwrap();

        assert_eq!(authz.username(), username);
        assert_eq!(authz.password(), password);
        Ok(())
    }

    #[test]
    fn bad_request_on_parse_error() {
        let mut headers = Headers::new();
        headers
            .insert(AUTHORIZATION, "<nori ate the tag. yum.>")
            .unwrap();
        let err = BasicAuth::from_headers(headers).unwrap_err();
        assert_eq!(err.status(), 400);
    }
}
