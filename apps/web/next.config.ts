import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  reactStrictMode: true,
  transpilePackages: ['@soldex/sdk'],
  // In Next.js 16, this is a top-level property
  serverExternalPackages: ['@coral-xyz/anchor'],
  // Setting an empty turbopack config silences the Webpack conflict error in Next.js 16
  // while allowing the use of the custom webpack configuration for fallbacks.
  turbopack: {},
  webpack: (config) => {
    config.resolve.fallback = {
      ...config.resolve.fallback,
      fs: false,
      net: false,
      tls: false,
      crypto: false,
      os: false,
      path: false,
      stream: false,
      url: false,
      zlib: false,
      http: false,
      https: false,
      assert: false,
      buffer: false,
    }
    return config
  },
  async headers() {
    return [
      {
        source: '/(.*)',
        headers: [
          { key: 'X-Frame-Options', value: 'DENY' },
          { key: 'X-Content-Type-Options', value: 'nosniff' },
          { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
          {
            key: 'Content-Security-Policy',
            value: [
              "default-src 'self'",
              "script-src 'self' 'unsafe-eval' 'unsafe-inline'",
              "style-src 'self' 'unsafe-inline' https://fonts.googleapis.com",
              "font-src 'self' https://fonts.gstatic.com",
              "connect-src 'self' wss: https: ws: localhost:*",
              "img-src 'self' data: https:",
            ].join('; '),
          },
        ],
      },
    ]
  },
}

export default nextConfig
