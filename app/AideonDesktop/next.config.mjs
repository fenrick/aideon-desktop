const devHost = process.env.TAURI_DEV_HOST ?? 'localhost';

/** @type {import("next").NextConfig} */
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  images: {
    unoptimized: true,
  },
  eslint: {
    ignoreDuringBuilds: true,
  },
  assetPrefix: process.env.NODE_ENV === 'development' ? `http://${devHost}:1420` : undefined,
};

export default nextConfig;
