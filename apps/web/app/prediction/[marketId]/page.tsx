'use client'
import React, { useEffect, useMemo } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { useMarketStore } from '@/hooks/useMarketStore'
import { PolyMarketDetail } from '@/components/polymarket/PolyMarketDetail'
import { motion, AnimatePresence } from 'framer-motion'

export default function MarketDetailPage() {
  const { marketId } = useParams()
  const router = useRouter()
  const { 
    polyOpportunities, 
    jupiterOpportunities,
    setSelectedPolyMarket,
    setSelectedJupiterMarket
  } = useMarketStore()

  const market = useMemo(() => {
    return (polyOpportunities.find(o => o.market_id === marketId) ||
            jupiterOpportunities.find(o => o.market_id === marketId))
  }, [marketId, polyOpportunities, jupiterOpportunities])

  useEffect(() => {
    if (market) {
      if ('question' in market) {
        setSelectedPolyMarket(market.market_id)
        setSelectedJupiterMarket(null)
      } else {
        setSelectedJupiterMarket(market.market_id)
        setSelectedPolyMarket(null)
      }
    }
  }, [market, setSelectedPolyMarket, setSelectedJupiterMarket])

  if (!market) {
    return (
      <div className="flex flex-col items-center justify-center py-40 opacity-50">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-white mb-4" />
        <p className="text-sm font-bold uppercase tracking-widest">Finding Market...</p>
      </div>
    )
  }

  return (
    <div className="max-w-[1400px] mx-auto px-6 py-8">
      <AnimatePresence mode="wait">
        <motion.div
          key={market.market_id}
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -20 }}
          transition={{ duration: 0.3 }}
        >
          <PolyMarketDetail 
            market={market} 
            onBack={() => router.push('/prediction')} 
          />
        </motion.div>
      </AnimatePresence>
    </div>
  )
}
