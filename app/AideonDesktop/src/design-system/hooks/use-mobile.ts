import * as React from 'react';

const MOBILE_BREAKPOINT = 768;

/**
 * Track whether viewport width is below the mobile breakpoint.
 * @returns True when the viewport is considered mobile.
 */
export function useIsMobile() {
  const [isMobile, setIsMobile] = React.useState(false);

  React.useEffect(() => {
    const maxWidth = String(MOBILE_BREAKPOINT - 1);
    const mql = globalThis.matchMedia(`(max-width: ${maxWidth}px)`);
    const onChange = () => {
      setIsMobile(globalThis.innerWidth < MOBILE_BREAKPOINT);
    };
    mql.addEventListener('change', onChange);
    setIsMobile(globalThis.innerWidth < MOBILE_BREAKPOINT);
    return () => {
      mql.removeEventListener('change', onChange);
    };
  }, []);

  return isMobile;
}
