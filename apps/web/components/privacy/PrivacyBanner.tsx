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
    <div className="flex items-center gap-3 px-4 py-2 bg-yellow-500/10 border-b border-yellow-500/20 text-xs shrink-0">
      <AlertTriangle className="w-3.5 h-3.5 text-yellow-400 shrink-0" />
      <span className="text-yellow-300"><strong>Privacy rotation recommended.</strong> Your wallet has been active 30+ days. Rotate to unlink your trading history.</span>
      <Link href="/rotate" className="ml-auto text-yellow-400 underline whitespace-nowrap hover:text-yellow-300">Rotate now →</Link>
      <button onClick={() => setDismissed(true)} className="text-yellow-600 hover:text-yellow-400"><X className="w-3.5 h-3.5" /></button>
    </div>
  )
}
