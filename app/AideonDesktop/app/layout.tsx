import type { Metadata } from 'next';

import '../src/styles.css';
import { AppProviders } from './providers';

export const metadata: Metadata = {
  title: 'Aideon',
  description: 'Aideon Desktop shell hosting Praxis workspaces.',
};

/**
 * Root document layout for the desktop renderer.
 * @param root0 - Layout props.
 * @param root0.children - Child nodes.
 */
export default function RootLayout({ children }: { readonly children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        <AppProviders>{children}</AppProviders>
      </body>
    </html>
  );
}
