import { TradingTerminal } from '@/components/terminal/TradingTerminal'

export default async function PerpsMarketPage({ 
  params 
}: { 
  params: Promise<{ market: string }> 
}) {
  const { market } = await params
  return <TradingTerminal defaultMarket={market} />
}

export function generateStaticParams() {
  return [
    { market: 'SOL-PERP' },
    { market: 'BTC-PERP' },
    { market: 'ETH-PERP' },
  ]
}
