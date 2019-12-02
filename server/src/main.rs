use futures::future;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::StatusCode;
use hyper::{Body, Client, Request, Response, Server};
use hyper_tls::HttpsConnector;

type BoxFut = Box<dyn Future<Item = Response<Body>, Error = hyper::Error> + Send>;

fn echo() -> impl Fn(Request<Body>) -> BoxFut {
    let https = HttpsConnector::new(4).expect("TLS initialization failed");
    let client = Client::builder().build::<_, hyper::Body>(https);

    move |req: Request<Body>| -> BoxFut {
        let mut response = Response::new(Body::empty());
        let path = req.uri().path();
        let path_prefix = "/proxy/portal-skyscapecloud-com/";
        if path.starts_with(path_prefix) {
            let request = Request::builder()
                .method(req.method())
                .uri(format!(
                    "https://portal.skyscapecloud.com/{}",
                    path.get(path_prefix.len()..path.len()).unwrap()
                ))
                .body(req.into_body())
                .expect("request builder");

            return Box::new(client.request(request).then(|response_result| {
                *response.body_mut() = match response_result {
                    Ok(res) => {
                        let (parts, body) = res.into_parts();
                        *response.headers_mut() = parts.headers;
                        *response.status_mut() = parts.status;
                        *response.version_mut() = parts.version;
                        Body::wrap_stream(body)
                    },
                    Err(err) => Body::from(format!("Error: {}", err)),
                };
                Ok(response)
            }));
        } else {
            *response.status_mut() = StatusCode::NOT_FOUND;
        };

        Box::new(future::ok(response))
    }
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr)
        .serve(|| service_fn(echo()))
        .map_err(|e| eprintln!("server error: {}", e));

    hyper::rt::run(server);
}
