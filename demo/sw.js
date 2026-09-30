// Offline support and updates.
// - The app shell and the WASM package are cached under a cache named after the build id, which demo/build.mjs
//   writes below (a hash of the shell, the wasm and the table manifest). A new build changes this file, so the
//   browser installs the new service worker, which caches the new shell and deletes the old shell caches.
// - The table manifest is fetched network-first (the cached copy is used only offline), so a rebuilt table is
//   never checked against a stale hash.
// - Tables are named by their hash and cached by the worker (Cache API) after their hash is checked.
const BUILD = "__BUILD_ID__";
const SHELL = `yatzy-advisor-shell-${BUILD}`;
const FILES = ["./", "index.html", "style.css", "app.js", "i18n.js", "worker.js", "pkg/yatzy_solver.js", "pkg/yatzy_solver_bg.wasm"];
const MANIFEST = "tables/manifest.json";

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(SHELL).then((c) => c.addAll(FILES.map((f) => new Request(f, { cache: "reload" })))).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("yatzy-advisor-shell-") && k !== SHELL).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (url.origin !== location.origin || e.request.method !== "GET") return;
  if (url.pathname.endsWith("/" + MANIFEST)) {
    // Network first, bypassing the HTTP cache; the cached copy only when offline.
    e.respondWith(
      fetch(e.request, { cache: "no-cache" })
        .then((res) => {
          if (res.ok) {
            const copy = res.clone();
            caches.open(SHELL).then((c) => c.put(e.request, copy));
          }
          return res;
        })
        .catch(() => caches.match(e.request).then((hit) => hit || Response.error())),
    );
    return;
  }
  if (url.pathname.includes("/tables/")) return; // the worker caches tables itself, by hash
  e.respondWith(caches.match(e.request).then((hit) => hit || fetch(e.request)));
});
