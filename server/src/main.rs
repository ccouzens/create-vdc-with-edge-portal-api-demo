use cookie::Cookie;
use http::header::HeaderMap;
use http::Uri;
use hyper::client::HttpConnector;
use hyper::service::make_service_fn;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper::{Body, Client, Request, Response, Server};
use hyper_tls::HttpsConnector;
use std::convert::Infallible;
use std::fmt::Display;
use std::future::Future;

fn split_path(path: &str) -> [Option<&str>; 3] {
    let mut indices = path.match_indices('/');
    let (_, b, c) = (
        indices.next(),
        indices.next().map(|i| i.0),
        indices.next().map(|i| i.0),
    );
    match (b, c) {
        (Some(b), Some(c)) => [path.get(1..b), path.get(b + 1..c), path.get(c..)],
        _ => [path.get(1..), None, None],
    }
}

#[test]
fn split_path_test() {
    assert_eq!(
        split_path("/proxy/portal.skyscapecloud.com/api/authenticate"),
        [
            Some("proxy"),
            Some("portal.skyscapecloud.com"),
            Some("/api/authenticate")
        ]
    );
    assert_eq!(split_path("/style.css"), [Some("style.css"), None, None]);
}

fn modify_set_cookie<'a: 'b, 'b>(
    path_base: &'a str,
) -> impl Fn(&'b http::header::HeaderValue) -> Option<http::header::HeaderValue> {
    move |header_value: &http::header::HeaderValue| {
        let cookie_str = header_value.to_str().ok()?;
        let mut cookie = Cookie::parse(cookie_str).ok()?;
        let path = cookie.path().unwrap_or("/");
        if !path.starts_with('/') {
            return None;
        }
        let full_path = format!("{}{}", path_base, path);
        cookie.set_path(full_path);
        cookie.set_secure(false);
        cookie.set_same_site(cookie::SameSite::Strict);
        Some(cookie.to_string().parse().ok()?)
    }
}

fn cleanse_set_cookies(path_base: &str, headers: &mut HeaderMap) {
    if let http::header::Entry::Occupied(entry) = headers.entry(http::header::SET_COOKIE) {
        let cookie_modifier = modify_set_cookie(path_base);
        let mut cookies: Vec<_> = entry.iter().filter_map(cookie_modifier).collect();
        headers.remove(http::header::SET_COOKIE);
        for cookie in cookies.drain(..) {
            headers.append(http::header::SET_COOKIE, cookie);
        }
    };
}

fn cleanse_response_headers(path_base: &str, headers: &mut HeaderMap) {
    headers.remove(http::header::STRICT_TRANSPORT_SECURITY);
    cleanse_set_cookies(path_base, headers);
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
    cleanse_response_headers("/proxy/portal-skyscapecloud-com", &mut headers);
    assert_eq!(headers, expected);
}

fn cleanse_request_headers(headers: &mut HeaderMap) {
    headers.remove(http::header::HOST);
}

fn not_found_response() -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::NOT_FOUND;
    Ok(response)
}

fn error_response<T: Display>(err: T) -> Result<Response<Body>, Infallible> {
    eprintln!("error: {}", err);
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    Ok(response)
}

async fn proxy_server_filter(
    mut request: Request<Body>,
    client: Client<HttpsConnector<HttpConnector>>,
) -> Result<Response<Body>, Infallible> {
    match request
        .uri()
        .path_and_query()
        .map(|path_and_query| split_path(path_and_query.as_str()))
    {
        Some([Some("proxy"), Some(proxied_server), Some(path_and_query)]) => {
            if !proxied_server.ends_with(".portal.skyscapecloud.com")
                && proxied_server != "portal.skyscapecloud.com"
            {
                return not_found_response();
            }

            match Uri::builder()
                .scheme("https")
                .authority(proxied_server)
                .path_and_query(path_and_query)
                .build()
            {
                Ok(client_url) => {
                    let cookie_base_path = format!("/proxy/{}", proxied_server);
                    cleanse_request_headers(request.headers_mut());
                    *request.uri_mut() = client_url;
                    match client.request(request).await {
                        Ok(mut response) => {
                            cleanse_response_headers(&cookie_base_path, response.headers_mut());
                            Ok(response)
                        }
                        Err(err) => error_response(err),
                    }
                }
                Err(err) => error_response(err),
            }
        }
        Some([Some(resource_name), None, None]) => {
            let resource = match resource_name {
                "" => Some((
                    include_str!("../../dist/index.html"),
                    "text/html; charset=UTF-8",
                )),
                "main.js" => Some((
                    include_str!("../../dist/main.js"),
                    "text/javascript; charset=UTF-8",
                )),
                "serviceWorker.js" => Some((
                    include_str!("../../dist/serviceWorker.js"),
                    "text/javascript; charset=UTF-8",
                )),
                _ => None,
            };
            match resource {
                Some((content, mime_type)) => {
                    let mut response = Response::new(Body::from(content));
                    response
                        .headers_mut()
                        .insert(http::header::CONTENT_TYPE, mime_type.parse().unwrap());
                    Ok(response)
                }
                None => not_found_response(),
            }
        }
        _ => not_found_response(),
    }
}

fn proxy_server(
    request: Request<Body>,
) -> impl Future<Output = Result<Response<Body>, Infallible>> {
    let client: Client<HttpsConnector<HttpConnector>> =
        Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

    proxy_server_filter(request, client)
}

#[tokio::main]
async fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(proxy_server)) });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
