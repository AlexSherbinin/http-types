// This is the compat file for the "hyperium/http" crate.

use crate::headers::{HeaderName, HeaderValue, Headers};
use crate::{Body, Error, Method, Request, Response, StatusCode, Url, Version};
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

impl From<http02::Method> for Method {
    fn from(method: http02::Method) -> Self {
        Method::from_str(method.as_str()).unwrap()
    }
}

impl From<Method> for http02::Method {
    fn from(method: Method) -> Self {
        http02::Method::from_str(method.as_ref()).unwrap()
    }
}

impl From<http02::StatusCode> for StatusCode {
    fn from(status: http02::StatusCode) -> Self {
        StatusCode::try_from(status.as_u16()).unwrap()
    }
}

impl From<StatusCode> for http02::StatusCode {
    fn from(status: StatusCode) -> Self {
        http02::StatusCode::from_u16(status.into()).unwrap()
    }
}

impl From<http02::Version> for Version {
    fn from(version: http02::Version) -> Self {
        match version {
            http02::Version::HTTP_09 => Version::Http0_9,
            http02::Version::HTTP_10 => Version::Http1_0,
            http02::Version::HTTP_11 => Version::Http1_1,
            http02::Version::HTTP_2 => Version::Http2_0,
            http02::Version::HTTP_3 => Version::Http3_0,
            _ => panic!("unknown HTTP version conversion"),
        }
    }
}

impl From<Version> for http02::Version {
    fn from(version: Version) -> Self {
        match version {
            Version::Http0_9 => http02::Version::HTTP_09,
            Version::Http1_0 => http02::Version::HTTP_10,
            Version::Http1_1 => http02::Version::HTTP_11,
            Version::Http2_0 => http02::Version::HTTP_2,
            Version::Http3_0 => http02::Version::HTTP_3,
        }
    }
}

impl TryFrom<http02::header::HeaderName> for HeaderName {
    type Error = Error;

    fn try_from(name: http02::header::HeaderName) -> Result<Self, Self::Error> {
        let name = name.as_str().as_bytes().to_owned();
        HeaderName::from_bytes(name)
    }
}

impl TryFrom<HeaderName> for http02::header::HeaderName {
    type Error = Error;

    fn try_from(name: HeaderName) -> Result<Self, Self::Error> {
        let name = name.as_str().as_bytes();
        http02::header::HeaderName::from_bytes(name).map_err(Error::new_adhoc)
    }
}

impl TryFrom<http02::header::HeaderValue> for HeaderValue {
    type Error = Error;

    fn try_from(value: http02::header::HeaderValue) -> Result<Self, Self::Error> {
        let value = value.as_bytes().to_owned();
        HeaderValue::from_bytes(value)
    }
}

impl TryFrom<HeaderValue> for http02::header::HeaderValue {
    type Error = Error;

    fn try_from(value: HeaderValue) -> Result<Self, Self::Error> {
        let value = value.as_str().as_bytes();
        http02::header::HeaderValue::from_bytes(value).map_err(Error::new_adhoc)
    }
}

impl TryFrom<http02::HeaderMap> for Headers {
    type Error = Error;

    fn try_from(hyperium_headers: http02::HeaderMap) -> Result<Self, Self::Error> {
        let mut headers = Headers::new();

        hyperium_headers
            .into_iter()
            .map(|(name, value)| {
                if let Some(name) = name {
                    let value: HeaderValue = value.try_into()?;
                    let name: HeaderName = name.try_into()?;
                    headers.append(name, value)?;
                }
                Ok(())
            })
            .collect::<Result<Vec<()>, Error>>()?;

        Ok(headers)
    }
}

impl TryFrom<Headers> for http02::HeaderMap {
    type Error = Error;

    fn try_from(headers: Headers) -> Result<Self, Self::Error> {
        let mut hyperium_headers = http02::HeaderMap::new();

        headers
            .into_iter()
            .map(|(name, values)| {
                let name: http02::header::HeaderName = name.try_into()?;

                values
                    .into_iter()
                    .map(|value| {
                        let value: http02::header::HeaderValue = value.try_into()?;
                        hyperium_headers.append(&name, value);
                        Ok(())
                    })
                    .collect::<Result<Vec<()>, Error>>()?;

                Ok(())
            })
            .collect::<Result<Vec<()>, Error>>()?;

        Ok(hyperium_headers)
    }
}

fn hyperium_headers_to_headers(
    hyperium_headers: http02::HeaderMap,
    headers: &mut Headers,
) -> crate::Result<()> {
    for (name, value) in hyperium_headers {
        let value = value.as_bytes().to_owned();
        let value = unsafe { HeaderValue::from_bytes_unchecked(value) };
        if let Some(name) = name {
            let name = name.as_str().as_bytes().to_owned();
            let name = unsafe { HeaderName::from_bytes_unchecked(name) };
            headers.append(name, value)?;
        }
    }
    Ok(())
}

fn headers_to_hyperium_headers(headers: &mut Headers, hyperium_headers: &mut http02::HeaderMap) {
    for (name, values) in headers {
        let name = format!("{}", name).into_bytes();
        let name = http02::header::HeaderName::from_bytes(&name).unwrap();

        for value in values.iter() {
            let value = format!("{}", value).into_bytes();
            let value = http02::header::HeaderValue::from_bytes(&value).unwrap();
            hyperium_headers.append(&name, value);
        }
    }
}

// Neither type is defined in this lib, so we can't do From/Into impls
fn from_uri_to_url(uri: http02::Uri) -> Result<Url, crate::url::ParseError> {
    format!("{}", uri).parse()
}

// Neither type is defined in this lib, so we can't do From/Into impls
fn from_url_to_uri(url: &Url) -> http02::Uri {
    http02::Uri::try_from(&format!("{}", url)).unwrap()
}

impl TryFrom<http02::Request<Body>> for Request {
    type Error = crate::Error;

    fn try_from(req: http02::Request<Body>) -> Result<Self, Self::Error> {
        let (parts, body) = req.into_parts();
        let method = parts.method.into();
        let url = from_uri_to_url(parts.uri)?;
        let mut req = Request::new(method, url);
        req.set_body(body);
        req.set_version(Some(parts.version.into()));
        hyperium_headers_to_headers(parts.headers, req.as_mut())?;
        Ok(req)
    }
}

impl From<Request> for http02::Request<Body> {
    fn from(mut req: Request) -> Self {
        let method: http02::Method = req.method().into();
        let version = req.version().map(|v| v.into()).unwrap_or_default();
        let mut builder = http02::request::Builder::new()
            .method(method)
            .uri(from_url_to_uri(req.url()))
            .version(version);
        headers_to_hyperium_headers(req.as_mut(), builder.headers_mut().unwrap());
        builder.body(req.into()).unwrap()
    }
}

impl TryFrom<http02::Response<Body>> for Response {
    type Error = crate::Error;
    fn try_from(res: http02::Response<Body>) -> Result<Self, Self::Error> {
        let (parts, body) = res.into_parts();
        let mut res = Response::new(parts.status);
        res.set_body(body);
        res.set_version(Some(parts.version.into()));
        hyperium_headers_to_headers(parts.headers, res.as_mut())?;
        Ok(res)
    }
}

impl From<Response> for http02::Response<Body> {
    fn from(mut res: Response) -> Self {
        let status: u16 = res.status().into();
        let version = res.version().map(|v| v.into()).unwrap_or_default();
        let mut builder = http02::response::Builder::new()
            .status(status)
            .version(version);
        headers_to_hyperium_headers(res.as_mut(), builder.headers_mut().unwrap());
        let body = res.take_body();
        builder.body(body).unwrap()
    }
}
