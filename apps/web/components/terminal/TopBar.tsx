'use client'
import React from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { SelfCustodyWalletButton } from '../wallet/SelfCustodyWalletButton'
import { Shield, Activity, AlertCircle } from 'lucide-react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'

export function TopBar({ connected }: { connected: boolean }) {
  const path = usePathname()
  const { polyOpportunities } = useMarketStore()
  const { rotationDue } = useSelfCustodyWallet()

  const nav = [
    { href: '/',           label: 'Perps' },
    { 
      href: '/polymarket', 
      label: (
        <span className="flex items-center gap-1.5">
          Polymarket
          {polyOpportunities.length > 0 && (
            <span className="bg-blue-500/10 text-[#3b82f6] text-[9px] px-1 py-0.5 rounded-full font-bold">
              {polyOpportunities.length}
            </span>
          )}
        </span>
      )
    },
    { href: '/portfolio',  label: 'Portfolio' },
    { 
      href: '/rotate',     
      label: (
        <span className="flex items-center gap-1.5">
          ⟳ Rotate
          {rotationDue && (
            <AlertCircle className="w-3 h-3 text-yellow-500 animate-pulse" />
          )}
        </span>
      )
    },
  ]
  return (
    <header className="flex items-center h-[44px] px-4 border-b border-[#1e2634] bg-[#0d1117] shrink-0 gap-0">
      <div className="flex items-center gap-2 px-4 border-r border-[#1e2634] h-full mr-4">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" className="text-[#3b82f6]">
          <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="2"/>
          <path d="M8 12l3 3 5-6" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
        </svg>
        <span className="font-bold text-[#e2e8f0] tracking-tight text-sm uppercase letter-spacing-[-0.3px]">Soldex</span>
      </div>
      <nav className="flex items-center gap-1 h-full">
        {nav.map(({ href, label }) => (
          <Link key={href} href={href}
            className={`px-4 h-full flex items-center text-xs transition-all duration-150 border-b-2 ${
              path === href 
                ? 'text-[#e2e8f0] font-semibold border-[#3b82f6]' 
                : 'text-[#64748b] border-transparent hover:text-[#8b90a8]'
            }`}>
            {label}
          </Link>
        ))}
      </nav>
      <div className="ml-auto flex items-center gap-3">
        <div className="flex items-center gap-2 px-3 py-1 rounded-md bg-[#1e2634] text-[10px] font-mono">
          <div className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-green-400' : 'bg-red-400'} animate-pulse`} />
          <span className={connected ? 'text-green-400' : 'text-red-400'}>
            {connected ? 'Engine Online' : 'Engine Offline'}
          </span>
        </div>
        <SelfCustodyWalletButton />
      </div>
    </header>
  )
}
