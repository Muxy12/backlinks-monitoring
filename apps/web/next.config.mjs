/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // ESLint is not configured yet; `next build` type-checks regardless.
  eslint: { ignoreDuringBuilds: true },
};
export default nextConfig;
