use cookie::Cookie;
use futures::future;
use http::header::HeaderMap;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper::{Body, Client, Request, Response, Server};
use hyper_tls::HttpsConnector;

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

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
    if let Ok(http::header::Entry::Occupied(entry)) = headers.entry(http::header::SET_COOKIE) {
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

fn echo() -> impl Fn(Request<Body>) -> BoxFut {
    let https = HttpsConnector::new(4).expect("TLS initialization failed");
    let client = Client::builder().build::<_, hyper::Body>(https);

    move |server_req: Request<Body>| -> BoxFut {
        let mut server_response = Response::new(Body::empty());
        match split_path(server_req.uri().path()) {
            Some(["", "proxy", "portal-skyscapecloud-com", path]) => {
                let mut client_req_builder = Request::builder();
                client_req_builder.method(server_req.method());
                client_req_builder.uri(format!("https://portal.skyscapecloud.com/{}", path));
                let (server_req_parts, body) = server_req.into_parts();
                if let Some(client_req_headers) = client_req_builder.headers_mut() {
                    *client_req_headers = server_req_parts.headers;
                    cleanse_client_headers(client_req_headers);
                }
                match client_req_builder.body(body) {
                    Ok(client_req) => {
                        Box::new(client.request(client_req).then(|client_response_result| {
                            match client_response_result {
                                Ok(client_response) => {
                                    let (parts, body) = client_response.into_parts();
                                    *server_response.headers_mut() = parts.headers;
                                    cleanse_server_headers(server_response.headers_mut());
                                    *server_response.status_mut() = parts.status;
                                    *server_response.version_mut() = parts.version;
                                    *server_response.body_mut() = Body::wrap_stream(body);
                                }
                                Err(err) => {
                                    eprintln!("client response error: {}", err);
                                    *server_response.status_mut() =
                                        StatusCode::INTERNAL_SERVER_ERROR;
                                }
                            };
                            Ok(server_response)
                        }))
                    }
                    Err(err) => {
                        *server_response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                        eprintln!("request builder error: {}", err);
                        Box::new(future::ok(server_response))
                    }
                }
            }
            _ => {
                *server_response.status_mut() = StatusCode::NOT_FOUND;
                Box::new(future::ok(server_response))
            }
        }
    }
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(echo()))
        .map_err(|e| eprintln!("server error: {}", e));

    hyper::rt::run(server);
}
