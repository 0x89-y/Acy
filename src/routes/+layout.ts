// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
export const ssr = false;
// Pure SPA: rely on the adapter-static fallback so dynamic routes such as
// /app/[source]/[id] resolve client-side inside the Tauri webview.
export const prerender = false;
