'use client'
import React, { useState } from 'react'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { Shield, RefreshCw, CheckCircle, ArrowRight, Lock, Eye, EyeOff } from 'lucide-react'
import { TopBar } from '@/components/terminal/TopBar'
import { useEngineWS } from '@/hooks/useEngineWS'
import toast from 'react-hot-toast'

type Step = 'intro' | 'confirm' | 'rotating' | 'done'

export default function RotatePage() {
  const { wallet, history, retire, generate, rotationDue, daysLeft } = useSelfCustodyWallet()
  const { connected } = useEngineWS()
  const [step, setStep] = useState<Step>('intro')
  const [retiredWallet, setRetiredWallet] = useState<string | null>(null)
  const [newPubkey, setNewPubkey] = useState<string | null>(null)
  const [showKey, setShowKey] = useState(false)

  const handleRotate = async () => {
    setStep('rotating')
    const old = wallet?.pubkey ?? null
    setRetiredWallet(old)
    await retire()
    const fresh = useSelfCustodyWallet.toString() // re-read after retire
    setNewPubkey(null) // will update via store
    setTimeout(() => {
      const cur = (window as any).__soldexWallet ?? 'new-address'
      setStep('done')
      toast.success('Wallet rotated successfully')
    }, 1500)
  }

  return (
    <div className="flex flex-col h-screen bg-[#0a0b0f]">
      <TopBar connected={connected} />
      <div className="flex-1 flex items-center justify-center p-8">
        <div className="w-full max-w-lg">
          <div className="text-center mb-8">
            <div className="inline-flex items-center justify-center w-14 h-14 rounded-full bg-blue-500/10 border border-blue-500/20 mb-4">
              <Shield className="w-7 h-7 text-blue-400" />
            </div>
            <h1 className="text-2xl font-bold text-white mb-2">Privacy Rotation</h1>
            <p className="text-[#8b90a8] text-sm max-w-sm mx-auto">
              Generate a fresh wallet to unlink your trading history. Your old positions stay intact.
            </p>
          </div>

          {step === 'intro' && (
            <div className="flex flex-col gap-4">
              {/* Status */}
              <div className={`p-4 rounded border ${rotationDue ? 'bg-yellow-500/5 border-yellow-500/20' : 'bg-green-500/5 border-green-500/20'}`}>
                <div className="flex items-center justify-between mb-2">
                  <span className="text-xs text-[#4b5068]">Current Wallet</span>
                  <span className={`text-xs font-mono ${rotationDue ? 'text-yellow-400' : 'text-green-400'}`}>
                    {rotationDue ? '⚠ Rotation due' : `✓ ${daysLeft} days left`}
                  </span>
                </div>
                <p className="font-mono text-xs text-[#8b90a8] break-all">{wallet?.pubkey ?? '—'}</p>
                <p className="text-[10px] text-[#4b5068] mt-1">
                  Created: {wallet ? new Date(wallet.createdAt).toLocaleDateString() : '—'}
                </p>
              </div>

              {/* What happens */}
              <div className="bg-[#13151d] rounded border border-[#1e2130] p-4">
                <h3 className="text-xs font-semibold text-white mb-3 uppercase tracking-wider">What happens during rotation</h3>
                <div className="flex flex-col gap-2.5">
                  {[
                    { icon: RefreshCw, text: 'A new keypair is generated in your browser', color: 'text-blue-400' },
                    { icon: Lock,      text: 'Old wallet is retired — no on-chain link to new one', color: 'text-green-400' },
                    { icon: Shield,    text: 'Rotation history kept locally for your reference', color: 'text-purple-400' },
                    { icon: Eye,       text: 'You manually transfer any remaining balance before retiring', color: 'text-yellow-400' },
                  ].map(({ icon: Icon, text, color }, i) => (
                    <div key={i} className="flex items-center gap-3 text-xs text-[#8b90a8]">
                      <Icon className={`w-3.5 h-3.5 shrink-0 ${color}`} />
                      {text}
                    </div>
                  ))}
                </div>
              </div>

              {/* History */}
              {history.length > 0 && (
                <div className="bg-[#13151d] rounded border border-[#1e2130] p-3">
                  <h3 className="text-[10px] text-[#4b5068] uppercase tracking-wider mb-2">Rotation History ({history.length})</h3>
                  {history.map((h, i) => (
                    <div key={i} className="flex justify-between text-[10px] py-1 border-t border-[#1e2130] first:border-0">
                      <span className="font-mono text-[#8b90a8]">{h.pubkey.slice(0, 8)}…{h.pubkey.slice(-6)}</span>
                      <span className="text-[#4b5068]">{new Date(h.retiredAt).toLocaleDateString()}</span>
                    </div>
                  ))}
                </div>
              )}

              <button onClick={() => setStep('confirm')}
                className="flex items-center justify-center gap-2 py-3 bg-blue-600 hover:bg-blue-500 text-white rounded font-semibold text-sm transition-colors">
                <RefreshCw className="w-4 h-4" /> Begin Rotation
              </button>

              {!wallet && (
                <button onClick={() => generate()} className="py-3 border border-[#1e2130] text-[#8b90a8] rounded text-sm hover:text-white transition-colors">
                  Generate First Wallet
                </button>
              )}
            </div>
          )}

          {step === 'confirm' && (
            <div className="flex flex-col gap-4">
              <div className="bg-yellow-500/5 border border-yellow-500/20 rounded p-4">
                <h3 className="text-sm font-semibold text-yellow-400 mb-2">Before you rotate</h3>
                <ul className="flex flex-col gap-1.5 text-xs text-[#8b90a8]">
                  <li>• Transfer any USDC balance to your new wallet after rotation</li>
                  <li>• Close or transfer any open perps positions</li>
                  <li>• Collect any pending Polymarket winnings</li>
                  <li>• The old private key will be removed from this session</li>
                </ul>
              </div>
              <div className="flex gap-3">
                <button onClick={() => setStep('intro')} className="flex-1 py-3 border border-[#1e2130] text-[#8b90a8] rounded text-sm hover:text-white">
                  Back
                </button>
                <button onClick={handleRotate} className="flex-1 flex items-center justify-center gap-2 py-3 bg-blue-600 hover:bg-blue-500 text-white rounded font-semibold text-sm">
                  Confirm Rotation <ArrowRight className="w-4 h-4" />
                </button>
              </div>
            </div>
          )}

          {step === 'rotating' && (
            <div className="flex flex-col items-center gap-4 py-8">
              <RefreshCw className="w-10 h-10 text-blue-400 animate-spin" />
              <p className="text-white font-semibold">Generating new keypair…</p>
              <p className="text-xs text-[#4b5068]">Securely creating fresh wallet in browser</p>
            </div>
          )}

          {step === 'done' && (
            <div className="flex flex-col gap-4">
              <div className="flex flex-col items-center py-6 gap-3">
                <CheckCircle className="w-12 h-12 text-green-400" />
                <h2 className="text-xl font-bold text-white">Rotation Complete</h2>
                <p className="text-xs text-[#8b90a8] text-center">
                  Your new wallet is ready. The previous address has been retired from this session.
                </p>
              </div>
              {retiredWallet && (
                <div className="bg-[#13151d] rounded border border-[#1e2130] p-3 text-xs">
                  <p className="text-[#4b5068] mb-1">Retired wallet</p>
                  <p className="font-mono text-[#8b90a8] break-all">{retiredWallet}</p>
                </div>
              )}
              <div className="bg-[#13151d] rounded border border-green-500/20 p-3 text-xs">
                <p className="text-[#4b5068] mb-1">New wallet</p>
                <p className="font-mono text-green-400 break-all">{wallet?.pubkey ?? '—'}</p>
              </div>
              <button onClick={() => { setStep('intro') }}
                className="py-3 bg-[#13151d] border border-[#1e2130] text-[#e2e4ef] rounded text-sm hover:text-white">
                Done
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
