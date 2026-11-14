import type { NextConfig } from "next";

const apiOrigin = process.env.LEAFLET_API_ORIGIN ?? "http://127.0.0.1:8080";

const nextConfig: NextConfig = {
  // Single-binary self-hosting: the rust server serves this export, same origin.
  output: "export",
  images: { unoptimized: true },
  trailingSlash: true,
  // trailingSlash would rewrite /api/v1/x to /api/v1/x/ and the rust router
  // matches exact paths, so every api call 404s in dev without this.
  skipTrailingSlashRedirect: true,
  // Dev only. Exports ignore custom routes; in prod both sit on one origin.
  async rewrites() {
    if (process.env.NODE_ENV === "production") return [];
    return [{ source: "/api/:path*", destination: `${apiOrigin}/api/:path*` }];
  },
};

export default nextConfig;
