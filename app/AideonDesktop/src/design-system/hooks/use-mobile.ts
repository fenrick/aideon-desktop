import * as React from 'react';

const MOBILE_BREAKPOINT = 768;

/**
 * Track whether the viewport is currently within the mobile breakpoint.
 * @returns True when the viewport is below the breakpoint.
 */
export function useIsMobile() {
  const [isMobile, setIsMobile] = React.useState(false);

  React.useEffect(() => {
    const maxWidth = MOBILE_BREAKPOINT - 1;
    const query = `(max-width: ${maxWidth.toString()}px)`;
    const mql = globalThis.matchMedia(query);
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
