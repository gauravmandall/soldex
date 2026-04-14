import { Space_Grotesk } from 'next/font/google'
import './globals.css'
import { Providers } from '@/providers/Providers'

const spaceGrotesk = Space_Grotesk({ subsets: ['latin'], weight: ['400', '500', '600', '700'] })

export const metadata: Metadata = {
  title: 'Soldex.fi — Privacy-First Perps on Solana',
  description: 'Execute perpetual futures and Polymarket orders with self-custody wallets. Privacy by design.',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className="dark">
      <body className={`${spaceGrotesk.className} bg-[#0a0b0f] text-[#e2e4ef] antialiased`}>
        <Providers>{children}</Providers>
      </body>
    </html>
  )
}
