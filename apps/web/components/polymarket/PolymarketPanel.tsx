'use client'
import React, { useState, useMemo } from 'react'
import { useMarketStore, PolyOpportunity, JupiterOpportunity } from '@/hooks/useMarketStore'
import { useEngineWS } from '@/hooks/useEngineWS'
import { Search, Globe, LayoutGrid, Activity, Zap } from 'lucide-react'
import { PolyMarketCard } from './PolyMarketCard'
import { JupiterMarketCard } from './JupiterMarketCard'
import { motion, AnimatePresence } from 'framer-motion'
import { useRouter } from 'next/navigation'

interface Props {
  initialCategory?: string
  hideHeader?: boolean
  searchOverride?: string
}

export function PolymarketPanel({ initialCategory = 'all', hideHeader = false, searchOverride = '' }: Props) {
  const router = useRouter()
  const { 
    polyOpportunities, 
    jupiterOpportunities, 
    selectedPolyMarketId, 
    setSelectedPolyMarket,
    selectedJupiterMarketId,
    setSelectedJupiterMarket
  } = useMarketStore()

  const filteredPoly = useMemo(() => {
    return polyOpportunities.filter(o => {
      const matchesCat = initialCategory === 'all' || 
                        initialCategory === 'live' || // For now, treat all poly opps as live
                        o.category.toLowerCase() === initialCategory.toLowerCase() ||
                        (initialCategory === 'trending' && o.volume_24h > 100000)
      const matchesSearch = !searchOverride || o.question.toLowerCase().includes(searchOverride.toLowerCase())
      return matchesCat && matchesSearch
    })
  }, [polyOpportunities, initialCategory, searchOverride])

  const filteredJupiter = useMemo(() => {
    return jupiterOpportunities.filter(o => {
      const matchesCat = initialCategory === 'all' || 
                        (initialCategory === 'live' && o.is_live) ||
                        o.category.toLowerCase() === initialCategory.toLowerCase() ||
                        (initialCategory === 'trending' && o.volume_24h > 10000)
      const matchesSearch = !searchOverride || o.title.toLowerCase().includes(searchOverride.toLowerCase())
      return matchesCat && matchesSearch
    })
  }, [jupiterOpportunities, initialCategory, searchOverride])

  const allMarkets = useMemo(() => {
    const combined = [...filteredPoly, ...filteredJupiter]
    return combined.sort((a, b) => {
        const volA = 'volume_24h' in a ? (typeof a.volume_24h === 'string' ? parseFloat(a.volume_24h) : a.volume_24h) : 0
        const volB = 'volume_24h' in b ? (typeof b.volume_24h === 'string' ? parseFloat(b.volume_24h) : b.volume_24h) : 0
        return volB - volA
    })
  }, [filteredPoly, filteredJupiter])

  return (
    <div className="flex flex-col gap-8">
      {!hideHeader && (
        <div className="flex items-center justify-between border-b border-[#1e2634] pb-4">
           <div className="flex items-center gap-3">
              <Globe className="w-5 h-5 text-blue-500" />
              <h2 className="text-xl font-bold text-white uppercase tracking-widest">Market Discovery</h2>
           </div>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
        <AnimatePresence mode="popLayout">
          {allMarkets.map((market) => (
            'question' in market ? (
              <PolyMarketCard 
                key={market.market_id}
                market={market}
                active={selectedPolyMarketId === market.market_id}
                onClick={() => {
                   router.push(`/prediction/${market.market_id}`)
                }}
              />
            ) : (
              <JupiterMarketCard 
                key={market.market_id}
                market={market}
                active={selectedJupiterMarketId === market.market_id}
                onClick={() => {
                   router.push(`/prediction/${market.market_id}`)
                }}
              />
            )
          ))}
        </AnimatePresence>
      </div>

      {allMarkets.length === 0 && (
        <div className="flex flex-col items-center justify-center py-40 border border-dashed border-[#1e2634] rounded-3xl opacity-30">
           <Activity className="w-16 h-16 mb-4 text-[#4a5568]" />
           <p className="text-sm font-bold uppercase tracking-widest text-[#4a5568]">No live markets found</p>
        </div>
      )}
    </div>
  )
}
