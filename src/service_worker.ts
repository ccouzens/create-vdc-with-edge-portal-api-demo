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
  requestUrl.pathname = `${requestUrl.host.replace(/\./g, "-")}${
    requestUrl.pathname
  }`;
  requestUrl.host = swSelf.location.host;
  requestUrl.protocol = swSelf.location.protocol;
  event.respondWith(fetch(requestUrl.href, request));
});
