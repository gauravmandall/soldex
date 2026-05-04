'use client'
import React from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { SelfCustodyWalletButton } from '../wallet/SelfCustodyWalletButton'
import { Shield, Activity, AlertCircle, Terminal, Cpu, Globe, BarChart3 } from 'lucide-react'
import { useMarketStore } from '@/hooks/useMarketStore'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'

export function TopBar({ connected }: { connected: boolean }) {
  const path = usePathname()
  const { polyOpportunities } = useMarketStore()
  const { rotationDue } = useSelfCustodyWallet()

  const nav = [
    { href: '/',           label: 'PERPS', icon: Cpu },
    { 
      href: '/prediction', 
      label: 'PREDICTIONS',
      icon: Globe,
      badge: polyOpportunities.length > 0 ? polyOpportunities.length : null
    },
    { href: '/portfolio',  label: 'PORTFOLIO', icon: BarChart3 },
    { 
      href: '/rotate',     
      label: 'ROTATE',
      icon: Activity,
      alert: rotationDue
    },
  ]

  return (
    <header className="flex items-center h-[44px] px-4 border-b border-[var(--bd)] bg-[var(--bg1)] shrink-0 gap-0 font-mono">
      <div className="flex items-center gap-2 pr-6 border-r border-[var(--bd)] h-full">
        <div className="flex items-center gap-2">
           <Terminal className="w-5 h-5 text-[var(--green)]" />
           <span className="text-[14px] font-bold tracking-[2px] uppercase">SOLDEX.<span className="text-[var(--green)]">FI</span></span>
        </div>  
      </div>

      <nav className="flex items-center h-full ml-4 gap-1">
        {nav.map(({ href, label, icon: Icon, badge, alert }) => {
          const active = href === '/' ? path === '/' : path.startsWith(href)
          return (
            <Link key={href} href={href}
              className={`px-4 h-full flex items-center gap-2 text-[10px] font-bold tracking-widest transition-all duration-150 border-b-2 ${
                active 
                  ? 'text-[var(--green)] border-[var(--green)] bg-[var(--bg2)]/50' 
                  : 'text-[var(--tx2)] border-transparent hover:text-[var(--tx)] hover:bg-[var(--bg2)]/30'
              }`}>
              <Icon className="w-3.5 h-3.5" />
              {label}
              {badge && (
                <span className="bg-[var(--blue)]/20 text-[var(--blue)] text-[9px] px-1.5 py-0.5 rounded-[2px] font-bold ml-1">
                  {badge}
                </span>
              )}
              {alert && (
                <AlertCircle className="w-3 h-3 text-[var(--amber)] animate-pulse" />
              )}
            </Link>
          )
        })}
      </nav>

      <div className="ml-auto flex items-center gap-6">
        <div className="flex items-center gap-4">
           <div className="flex items-center gap-2 px-3 py-1 border border-[var(--bd)] bg-[var(--bg2)]">
              <div className={`w-1.5 h-1.5 rounded-full ${connected ? 'bg-[var(--green)] shadow-[0_0_8px_var(--green)]' : 'bg-[var(--red)]'} animate-pulse`} />
              <span className={`text-[9px] font-bold uppercase tracking-tighter ${connected ? 'text-[var(--green)]' : 'text-[var(--red)]'}`}>
                {connected ? 'Engine Online' : 'Engine Offline'}
              </span>
           </div>
           
           <div className="flex items-center gap-2 text-[10px] text-[var(--tx3)] uppercase">
              <Activity className="w-3 h-3" />
              <span>Network: <span className="text-[var(--tx)] font-bold">Mainnet</span></span>
           </div>
        </div>

        <div className="w-[1px] h-4 bg-[var(--bd)]" />
        
        <SelfCustodyWalletButton />
      </div>
    </header>
  )
}
