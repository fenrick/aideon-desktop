import '@testing-library/jest-dom/vitest';

// Vitest occasionally runs pending React scheduler callbacks after the jsdom
// environment has been torn down (seen on macOS runners), which leaves
// `window` undefined and trips ReactDOM's access to `window.event`. Keep a
// minimal stub so late callbacks don't throw even if jsdom has already cleaned
// itself up.
if (typeof globalThis.window === 'undefined') {
  (globalThis as unknown as { window: typeof globalThis & { event?: unknown } }).window =
    globalThis as typeof globalThis & { event?: unknown };
  if (typeof (globalThis as { document?: Document }).document === 'undefined') {
    // Document is rarely touched in these late callbacks, but provide a stub for safety.
    (globalThis as unknown as { document: Partial<Document> }).document = {};
  }
}

if (typeof globalThis.ResizeObserver === 'undefined') {
  class ResizeObserverFallback implements ResizeObserver {
    private readonly callback: ResizeObserverCallback;

    constructor(callback: ResizeObserverCallback) {
      this.callback = callback;
    }

    observe(target: Element) {
      this.callback(
        [
          {
            target,
            contentRect: {
              width: 1000,
              height: 800,
              x: 0,
              y: 0,
              top: 0,
              left: 0,
              right: 1000,
              bottom: 800,
              toJSON() {
                return {};
              },
            },
          } as ResizeObserverEntry,
        ],
        this,
      );
    }

    unobserve() {}

    disconnect() {}

    takeRecords(): ResizeObserverEntry[] {
      return [];
    }
  }

  (globalThis as unknown as { ResizeObserver: typeof ResizeObserver }).ResizeObserver =
    ResizeObserverFallback;
}

if (typeof window !== 'undefined' && typeof window.matchMedia !== 'function') {
  const mediaQueryMock: MediaQueryList = {
    matches: false,
    media: '',
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent() {
      return false;
    },
  };

  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({ ...mediaQueryMock, media: query }),
  });
}

if (typeof Element !== 'undefined' && typeof Element.prototype.scrollIntoView !== 'function') {
  Object.defineProperty(Element.prototype, 'scrollIntoView', {
    configurable: true,
    writable: true,
    value() {},
  });
}
