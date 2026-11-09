// One shared IntersectionObserver for "do something once this is on screen"
// (lazy icon loading). Cheaper than an observer per card in long lists.

const callbacks = new Map<Element, () => void>();
let observer: IntersectionObserver | null = null;

function shared(): IntersectionObserver | null {
  if (typeof IntersectionObserver === 'undefined') return null;
  observer ??= new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        const cb = callbacks.get(e.target);
        callbacks.delete(e.target);
        observer?.unobserve(e.target);
        cb?.();
      }
    },
    // Start a little before the card scrolls in. (Only extends the viewport;
    // inner scroll panes still clip, so those load as they come into view.)
    { rootMargin: '200px' }
  );
  return observer;
}

/** Calls `cb` once, the first time `node` is on (or near) the screen. Returns
 * a cleanup that stops watching. Without IntersectionObserver, calls it now. */
export function onceVisible(node: Element, cb: () => void): () => void {
  const io = shared();
  if (!io) {
    cb();
    return () => {};
  }
  callbacks.set(node, cb);
  io.observe(node);
  return () => {
    if (callbacks.delete(node)) io.unobserve(node);
  };
}
