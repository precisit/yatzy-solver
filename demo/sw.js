// Offline support: the app shell and the WASM package are cached on install and served cache-first. Tables are
// cached by the worker (Cache API) after their hash is checked.
const SHELL = "yatzy-advisor-shell-v1";
const FILES = ["./", "index.html", "style.css", "app.js", "i18n.js", "worker.js", "pkg/yatzy_solver.js", "pkg/yatzy_solver_bg.wasm", "tables/manifest.json"];

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(SHELL).then((c) => c.addAll(FILES)).then(() => self.skipWaiting()));
});
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));
self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (url.origin !== location.origin || url.pathname.includes("/tables/") && !url.pathname.endsWith("manifest.json")) return;
  e.respondWith(
    caches.match(e.request).then((hit) => hit || fetch(e.request).then((res) => {
      if (res.ok) caches.open(SHELL).then((c) => c.put(e.request, res.clone()));
      return res;
    })),
  );
});
