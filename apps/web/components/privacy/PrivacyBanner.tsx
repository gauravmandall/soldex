'use client'
import React, { useState } from 'react'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { AlertTriangle, X } from 'lucide-react'
import Link from 'next/link'

export function PrivacyBanner() {
  const { rotationDue, wallet } = useSelfCustodyWallet()
  const [dismissed, setDismissed] = useState(false)
  if (!wallet || !rotationDue || dismissed) return null
  return (
    <div className="flex items-center gap-3 px-4 h-[34px] bg-[#1e2634] border-b border-[#3b82f6]/20 text-[11px] shrink-0">
      <AlertTriangle className="w-3.5 h-3.5 text-yellow-500 shrink-0" />
      <span className="text-[#e2e8f0]">
        <span className="font-bold text-yellow-500 mr-2">PRIVACY ROTATION DUE</span>
        Your wallet has been active 30+ days. Rotate keys to unlink history.
      </span>
      <Link href="/rotate" className="ml-auto flex items-center gap-1.5 px-3 py-1 bg-[#3b82f6]/10 text-[#3b82f6] hover:bg-[#3b82f6]/20 rounded font-bold uppercase tracking-wider transition-colors">
        Rotate now →
      </Link>
      <button onClick={() => setDismissed(true)} className="text-[#64748b] hover:text-[#e2e8f0] ml-2">
        <X className="w-3.5 h-3.5" />
      </button>
    </div>
  )
}
