use futures::future;
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

fn echo() -> impl Fn(Request<Body>) -> BoxFut {
    let https = HttpsConnector::new(4).expect("TLS initialization failed");
    let client = Client::builder().build::<_, hyper::Body>(https);

    move |server_req: Request<Body>| -> BoxFut {
        let mut server_response = Response::new(Body::empty());
        match split_path(server_req.uri().path()) {
            Some(["", "proxy", "portal-skyscapecloud-com", path]) => {
                match Request::builder()
                    .method(server_req.method())
                    .uri(format!("https://portal.skyscapecloud.com/{}", path))
                    .body(server_req.into_body())
                {
                    Ok(client_req) => {
                        Box::new(client.request(client_req).then(|client_response_result| {
                            match client_response_result {
                                Ok(client_response) => {
                                    let (parts, body) = client_response.into_parts();
                                    *server_response.headers_mut() = parts.headers;
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
