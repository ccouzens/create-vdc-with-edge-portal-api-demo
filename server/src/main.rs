use cookie::Cookie;
use http::header::HeaderMap;
use http::Uri;
use hyper::service::make_service_fn;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper::{Body, Client, Request, Response, Server};
use hyper_tls::HttpsConnector;
use std::convert::Infallible;
use std::fmt::Display;

fn split_path(path: &str) -> Option<[&str; 3]> {
    let mut indices = path.match_indices('/');
    let (_, b, c) = (indices.next()?.0, indices.next()?.0, indices.next()?.0);
    Some([path.get(..b)?, path.get(b..c)?, path.get(c..)?])
}

#[test]
fn split_path_test() {
    assert_eq!(
        split_path("/proxy/portal-skyscapecloud-com/api/authenticate"),
        Some(["/proxy", "/portal-skyscapecloud-com", "/api/authenticate"])
    );
}

fn cleanse_set_cookie(
    header_value: &http::header::HeaderValue,
) -> Option<http::header::HeaderValue> {
    let cookie_str = header_value.to_str().ok()?;
    let mut cookie = Cookie::parse(cookie_str).ok()?;
    let path = cookie.path().unwrap_or("/");
    if !path.starts_with('/') {
        return None;
    }
    let full_path = format!(
        "/proxy/portal-skyscapecloud-com/{}",
        path.get(1..).unwrap_or("")
    );
    cookie.set_path(full_path);
    cookie.set_secure(false);
    cookie.set_same_site(cookie::SameSite::Strict);
    Some(cookie.to_string().parse().ok()?)
}

fn cleanse_set_cookies(headers: &mut HeaderMap) {
    if let http::header::Entry::Occupied(entry) = headers.entry(http::header::SET_COOKIE) {
        let mut cookies: Vec<_> = entry.iter().filter_map(cleanse_set_cookie).collect();
        headers.remove(http::header::SET_COOKIE);
        for cookie in cookies.drain(..) {
            headers.append(http::header::SET_COOKIE, cookie);
        }
    };
}

fn cleanse_response_headers(headers: &mut HeaderMap) {
    headers.remove(http::header::STRICT_TRANSPORT_SECURITY);
    cleanse_set_cookies(headers);
}

#[test]
fn cleanse_response_headers_test() {
    let mut headers = HeaderMap::new();
    headers.append("content-type", "application/json".parse().unwrap());
    headers.append(
        "set-cookie",
        "_session=f81; Path=/api; HttpOnly; Expires=Wed, 11 Dec 2019 21:47:38 GMT; secure"
            .parse()
            .unwrap(),
    );
    headers.append(
        "strict-transport-security",
        "max-age=31536000".parse().unwrap(),
    );

    let mut expected = HeaderMap::new();
    expected.append("content-type", "application/json".parse().unwrap());
    expected.append("set-cookie", "_session=f81; HttpOnly; SameSite=Strict; Path=/proxy/portal-skyscapecloud-com/api; Expires=Wed, 11 Dec 2019 21:47:38 GMT".parse().unwrap());
    cleanse_response_headers(&mut headers);
    assert_eq!(headers, expected);
}

fn cleanse_request_headers(headers: &mut HeaderMap) {
    headers.remove(http::header::HOST);
}

fn error_response<T: Display>(err: T) -> Result<Response<Body>, Infallible> {
    eprintln!("error: {}", err);
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    Ok(response)
}

async fn echo(mut request: Request<Body>) -> Result<Response<Body>, Infallible> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    match split_path(request.uri().path()) {
        Some(["/proxy", "/portal-skyscapecloud-com", path]) => {
            match Uri::builder()
                .scheme("https")
                .authority("portal.skyscapecloud.com")
                .path_and_query(path)
                .build()
            {
                Ok(client_url) => {
                    cleanse_request_headers(request.headers_mut());
                    *request.uri_mut() = client_url;
                    match client.request(request).await {
                        Ok(mut response) => {
                            cleanse_response_headers(response.headers_mut());
                            Ok(response)
                        }
                        Err(err) => error_response(err),
                    }
                }
                Err(err) => error_response(err),
            }
        }
        _ => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_FOUND;
            Ok(response)
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(echo)) });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
