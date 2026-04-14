import { TradingTerminal } from '@/components/terminal/TradingTerminal'

export default function PerpsMarketPage({ params }: { params: { market: string } }) {
  return <TradingTerminal defaultMarket={params.market} />
}

export function generateStaticParams() {
  return [
    { market: 'SOL-PERP' },
    { market: 'BTC-PERP' },
    { market: 'ETH-PERP' },
  ]
}
