use cookie::Cookie;
use http::header::HeaderMap;
use hyper::service::make_service_fn;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper::{Body, Client, Request, Response, Server};
use hyper_tls::HttpsConnector;
use std::convert::Infallible;

fn split_path(path: &str) -> Option<[&str; 4]> {
    let mut path_parts = path.splitn(4, '/');
    Some([
        path_parts.next()?,
        path_parts.next()?,
        path_parts.next()?,
        path_parts.next()?,
    ])
}

#[test]
fn split_path_test() {
    assert_eq!(
        split_path("/proxy/portal-skyscapecloud-com/api/authenticate"),
        Some(["", "proxy", "portal-skyscapecloud-com", "api/authenticate"])
    );
}

fn cleanse_server_headers(headers: &mut HeaderMap) {
    headers.remove(http::header::STRICT_TRANSPORT_SECURITY);
    let mut cookies = Vec::new();
    if let http::header::Entry::Occupied(entry) = headers.entry(http::header::SET_COOKIE) {
        for value in entry.iter() {
            if let Ok(svalue) = value.to_str() {
                if let Ok(mut cookie) = Cookie::parse(svalue) {
                    let path = cookie.path().unwrap_or("/");
                    if !path.starts_with('/') {
                        continue;
                    }
                    let full_path = format!(
                        "/proxy/portal-skyscapecloud-com/{}",
                        path.get(1..).unwrap_or("")
                    );
                    cookie.set_path(full_path);

                    cookie.set_secure(false);
                    cookie.set_same_site(cookie::SameSite::Strict);
                    if let Ok(cookie_header) = cookie.to_string().parse() {
                        cookies.push(cookie_header);
                    };
                };
            };
        }
    };
    headers.remove(http::header::SET_COOKIE);
    for cookie in cookies.drain(..) {
        headers.append(http::header::SET_COOKIE, cookie);
    }
}

#[test]
fn cleanse_server_headers_test() {
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
    cleanse_server_headers(&mut headers);
    assert_eq!(headers, expected);
}

fn cleanse_client_headers(headers: &mut HeaderMap) {
    headers.remove(http::header::HOST);
}

async fn echo(server_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let (server_req_parts, body) = server_req.into_parts();

    match split_path(server_req_parts.uri.path()) {
        Some(["", "proxy", "portal-skyscapecloud-com", path]) => {
            let mut client_req_builder = Request::builder()
                .method(server_req_parts.method)
                .uri(format!("https://portal.skyscapecloud.com/{}", path));
            if let Some(client_req_headers) = client_req_builder.headers_mut() {
                *client_req_headers = server_req_parts.headers;
                cleanse_client_headers(client_req_headers);
            }
            match client_req_builder.body(body) {
                Ok(client_req) => match client.request(client_req).await {
                    Ok(client_response) => {
                        let (parts, body) = client_response.into_parts();
                        let mut server_response = Response::new(Body::wrap_stream(body));

                        *server_response.headers_mut() = parts.headers;
                        cleanse_server_headers(server_response.headers_mut());
                        *server_response.status_mut() = parts.status;
                        *server_response.version_mut() = parts.version;
                        Ok(server_response)
                    }
                    Err(err) => {
                        eprintln!("client response error: {}", err);
                        Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap())
                    }
                },
                Err(err) => {
                    eprintln!("request builder error: {}", err);
                    Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::empty())
                        .unwrap())
                }
            }
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
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
