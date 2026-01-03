import Image from 'next/image';
import type { ReactElement } from 'react';

import styles from './splash-screen.module.css';

export interface SplashScreenProperties {
  /** Whether the splash screen is visible. */
  readonly visible?: boolean;
  /** Accept but don't display (used for timing/context). */
  readonly seconds?: number;
  /** Loading status text. */
  readonly line?: string;
}

/**
 * Full-screen splash overlay for Aideon Desktop.
 * @param root0
 * @param root0.visible
 * @param root0.seconds
 * @param root0.line
 */
export function SplashScreen({
  visible = true,
  seconds: _seconds = 0,
  line = 'Starting…',
}: SplashScreenProperties): ReactElement {
  if (!visible) {
    return (
      <>
        {undefined}
        {undefined}
      </>
    );
  }

  return (
    <div className={styles.splash}>
      <Image src="/splash.png" alt="" className={styles.bg} fill sizes="100vw" priority />

      <div className={styles.right}>
        <h1 className={styles.title}>Aideon&nbsp;Praxis</h1>

        <div className={styles.loading}>
          <span id="loadline">{line}</span>
          <div className={styles.bar}>
            <div className={styles.barInner} />
          </div>
        </div>
      </div>
    </div>
  );
}

export default SplashScreen;
