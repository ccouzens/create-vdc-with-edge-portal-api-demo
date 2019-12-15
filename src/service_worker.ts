/// <reference lib="webworker" />
const swSelf = (self as unknown) as ServiceWorkerGlobalScope;

swSelf.addEventListener("fetch", function(event) {
  const request = event.request;
  const requestUrl = new URL(request.url);
  if (
    request.mode === "navigate" ||
    requestUrl.hostname === swSelf.location.hostname
  ) {
    return;
  }
  requestUrl.pathname = `/proxy/${requestUrl.host}${requestUrl.pathname}`;
  requestUrl.host = swSelf.location.host;
  requestUrl.protocol = swSelf.location.protocol;
  event.respondWith(
    request.blob().then(body =>
      fetch(requestUrl.href, {
        method: request.method,
        headers: request.headers,
        body: body.size == 0 ? null : body,
        mode: request.mode,
        credentials: request.credentials,
        cache: request.cache,
        redirect: request.redirect,
        referrer: request.referrer,
        integrity: request.integrity
      })
    )
  );
});
