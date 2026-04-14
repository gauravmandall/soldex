'use client'
import React from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { SelfCustodyWalletButton } from '../wallet/SelfCustodyWalletButton'
import { Shield, Activity } from 'lucide-react'

export function TopBar({ connected }: { connected: boolean }) {
  const path = usePathname()
  const nav = [
    { href: '/',           label: 'Perps' },
    { href: '/polymarket', label: 'Polymarket' },
    { href: '/portfolio',  label: 'Portfolio' },
    { href: '/rotate',     label: '⟳ Rotate' },
  ]
  return (
    <header className="flex items-center h-11 px-4 border-b border-[#1e2130] bg-[#0a0b0f] shrink-0 gap-6">
      <div className="flex items-center gap-2 mr-2">
        <Shield className="w-4 h-4 text-blue-400" strokeWidth={1.5} />
        <span className="font-bold text-white tracking-tight text-sm">Soldex<span className="text-blue-400">.fi</span></span>
      </div>
      <nav className="flex items-center gap-1">
        {nav.map(({ href, label }) => (
          <Link key={href} href={href}
            className={`px-3 py-1 text-xs rounded transition-colors ${
              path === href ? 'text-white bg-[#1e2130]' : 'text-[#4b5068] hover:text-[#8b90a8]'
            }`}>
            {label}
          </Link>
        ))}
      </nav>
      <div className="ml-auto flex items-center gap-3">
        <div className="flex items-center gap-1 text-[10px]">
          <Activity className={`w-3 h-3 ${connected ? 'text-green-400' : 'text-red-400'}`} />
          <span className={connected ? 'text-green-400' : 'text-red-400'}>
            {connected ? 'Engine' : 'Offline'}
          </span>
        </div>
        <SelfCustodyWalletButton />
      </div>
    </header>
  )
}
