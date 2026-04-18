'use client'
import React, { useState } from 'react'
import { useSelfCustodyWallet } from '@/hooks/useSelfCustodyWallet'
import { Shield, RefreshCw, CheckCircle, ArrowRight, Lock, Eye, EyeOff, Terminal, Zap } from 'lucide-react'
import { TopBar } from '@/components/terminal/TopBar'
import { useEngineWS } from '@/hooks/useEngineWS'
import toast from 'react-hot-toast'

type Step = 'intro' | 'confirm' | 'rotating' | 'done'

export default function RotatePage() {
  const { wallet, history, retire, generate, rotationDue, daysLeft } = useSelfCustodyWallet()
  const { connected } = useEngineWS()
  const [step, setStep] = useState<Step>('intro')
  const [retiredWallet, setRetiredWallet] = useState<string | null>(null)

  const handleRotate = async () => {
    setStep('rotating')
    const old = wallet?.pubkey ?? null
    setRetiredWallet(old)
    await retire()
    setTimeout(() => {
      setStep('done')
      toast.success('Wallet rotated successfully')
    }, 1500)
  }

  return (
    <div className="flex flex-col h-screen bg-[var(--bg)] text-[var(--tx)] font-mono selection:bg-blue-500/30">
      <TopBar connected={connected} />
      
      <div className="flex-1 flex items-center justify-center p-8 no-scrollbar overflow-y-auto">
        <div className="w-full max-w-xl bg-[var(--bg1)] border border-[var(--bd)] p-10 shadow-2xl relative overflow-hidden">
          {/* Background decoration */}
          <div className="absolute top-0 right-0 p-4 opacity-5">
            <Terminal className="w-32 h-32" />
          </div>

          <div className="relative z-10">
            <div className="flex items-center gap-4 mb-8">
              <div className="w-12 h-12 border border-[var(--blue)] bg-[var(--blue)]/10 flex items-center justify-center text-[var(--blue)]">
                <Shield className="w-6 h-6" />
              </div>
              <div className="flex flex-col">
                <h1 className="text-[20px] font-bold text-white uppercase tracking-widest leading-none">Privacy Rotation</h1>
                <p className="text-[9px] text-[var(--tx3)] uppercase tracking-tighter mt-1">Generate fresh identity to unlink trading history</p>
              </div>
            </div>

            {step === 'intro' && (
              <div className="flex flex-col gap-6">
                {/* Status Box */}
                <div className={`p-5 border border-[var(--bd)] ${rotationDue ? 'bg-[var(--amber)]/5' : 'bg-[var(--green)]/5'}`}>
                  <div className="flex items-center justify-between mb-3">
                    <span className="text-[10px] text-[var(--tx3)] font-bold uppercase tracking-widest">Active Identity</span>
                    <span className={`text-[9px] font-bold uppercase tracking-widest px-2 py-0.5 rounded-[2px] ${rotationDue ? 'bg-[var(--amber)]/20 text-[var(--amber)]' : 'bg-[var(--green)]/20 text-[var(--green)]'}`}>
                      {rotationDue ? 'Rotation Required' : `Identity Stable (${daysLeft}D)`}
                    </span>
                  </div>
                  <p className="text-[11px] text-[var(--tx2)] break-all font-bold opacity-80 mb-2">{wallet?.pubkey ?? 'NO IDENTITY ACTIVE'}</p>
                  <div className="flex items-center gap-4">
                     <span className="text-[9px] text-[var(--tx3)] uppercase font-bold tracking-tighter">Created: {wallet ? new Date(wallet.createdAt).toLocaleDateString() : '—'}</span>
                  </div>
                </div>

                {/* Workflow info */}
                <div className="grid grid-cols-2 gap-4">
                  {[
                    { icon: RefreshCw, title: 'Key Generation', desc: 'Secure browser-side keypair', color: 'text-[var(--blue)]' },
                    { icon: Lock, title: 'Identity Retired', desc: 'Zero on-chain linkage', color: 'text-[var(--green)]' },
                    { icon: Shield, title: 'Local Log', desc: 'History kept on this device', color: 'text-[var(--purple)]' },
                    { icon: Eye, title: 'Manual Transfer', desc: 'Sweep balance before shift', color: 'text-[var(--amber)]' },
                  ].map(({ icon: Icon, title, desc, color }, i) => (
                    <div key={i} className="p-3 border border-[var(--bd)]/50 bg-[var(--bg2)]/30 flex gap-3">
                      <Icon className={`w-4 h-4 shrink-0 mt-0.5 ${color}`} />
                      <div className="flex flex-col">
                        <span className="text-[10px] font-bold text-[var(--tx2)] uppercase tracking-tight">{title}</span>
                        <span className="text-[8px] text-[var(--tx3)] uppercase leading-tight mt-0.5">{desc}</span>
                      </div>
                    </div>
                  ))}
                </div>

                {/* History */}
                {history.length > 0 && (
                  <div className="bg-[var(--bg2)]/50 border border-[var(--bd)] p-4">
                    <h3 className="text-[10px] text-[var(--tx3)] uppercase tracking-widest font-bold mb-3 flex items-center justify-between">
                      Rotation History 
                      <span className="text-[var(--tx2)]">[{history.length}]</span>
                    </h3>
                    <div className="flex flex-col gap-2 max-h-[120px] overflow-y-auto no-scrollbar">
                      {history.slice().reverse().map((h, i) => (
                        <div key={i} className="flex justify-between items-center text-[10px] py-2 border-t border-[var(--bd)]/50 first:border-0 transition-colors hover:bg-[var(--bg2)] px-1">
                          <span className="font-mono text-[var(--tx2)] truncate max-w-[200px]">{h.pubkey}</span>
                          <span className="text-[var(--tx3)] font-bold">{new Date(h.retiredAt).toLocaleDateString()}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                <div className="flex flex-col gap-3 mt-4">
                  <button onClick={() => setStep('confirm')}
                    className="w-full py-4 bg-[var(--blue)] text-white font-bold text-[11px] uppercase tracking-[3px] hover:bg-[var(--blue)]/90 transition-all flex items-center justify-center gap-2 group">
                    Begin Identity Shift <ArrowRight className="w-4 h-4 group-hover:translate-x-1 transition-transform" />
                  </button>
                  {!wallet && (
                    <button onClick={() => generate()} className="w-full py-3 border border-[var(--bd)] text-[var(--tx3)] hover:text-[var(--tx)] text-[10px] uppercase font-bold tracking-widest">
                      Initialize First Identity
                    </button>
                  )}
                </div>
              </div>
            )}

            {step === 'confirm' && (
              <div className="flex flex-col gap-6">
                <div className="p-5 border border-[var(--amber)]/30 bg-[var(--amber)]/5 flex flex-col gap-3">
                  <h3 className="text-[11px] font-bold text-[var(--amber)] uppercase tracking-widest flex items-center gap-2">
                    <Zap className="w-4 h-4" /> Critical Precautions
                  </h3>
                  <ul className="flex flex-col gap-2 text-[10px] text-[var(--tx2)] uppercase font-bold opacity-80">
                    <li className="flex gap-2"><span>•</span> Transfer remaining USDC to new address after rotation</li>
                    <li className="flex gap-2"><span>•</span> Close or settle all active perp positions</li>
                    <li className="flex gap-2"><span>•</span> Settle any outstanding prediction outcomes</li>
                    <li className="flex gap-2"><span>•</span> Current private key will be purged from active session</li>
                  </ul>
                </div>
                <div className="flex gap-4">
                  <button onClick={() => setStep('intro')} className="flex-1 py-4 border border-[var(--bd)] text-[var(--tx3)] hover:text-white uppercase text-[10px] font-bold tracking-widest">
                    Abort
                  </button>
                  <button onClick={handleRotate} className="flex-1 py-4 bg-[var(--blue)] text-white font-bold text-[10px] uppercase tracking-widest">
                    Confirm Shift
                  </button>
                </div>
              </div>
            )}

            {step === 'rotating' && (
              <div className="flex flex-col items-center gap-6 py-12">
                <RefreshCw className="w-12 h-12 text-[var(--blue)] animate-spin" />
                <div className="flex flex-col items-center gap-1">
                   <p className="text-white font-bold text-[14px] uppercase tracking-widest">Generating Identity</p>
                   <p className="text-[9px] text-[var(--tx3)] uppercase font-bold tracking-tighter">Harvesting local entropy…</p>
                </div>
              </div>
            )}

            {step === 'done' && (
              <div className="flex flex-col gap-6">
                <div className="flex flex-col items-center py-6 gap-4">
                  <div className="w-16 h-16 rounded-full border border-[var(--green)] bg-[var(--green)]/10 flex items-center justify-center">
                    <CheckCircle className="w-8 h-8 text-[var(--green)]" />
                  </div>
                  <div className="flex flex-col items-center gap-1">
                    <h2 className="text-[20px] font-bold text-white uppercase tracking-widest">Shift Complete</h2>
                    <p className="text-[9px] text-[var(--tx3)] uppercase font-bold tracking-tighter text-center max-w-[300px]">
                      Your identity has been successfully rotated. The previous keypair has been retired from this session.
                    </p>
                  </div>
                </div>
                <div className="flex flex-col gap-3">
                  {retiredWallet && (
                    <div className="p-3 border border-[var(--bd)] bg-[var(--bg2)]/30">
                      <p className="text-[8px] text-[var(--tx3)] uppercase font-bold mb-1 tracking-widest">Retired Identity</p>
                      <p className="font-bold text-[10px] text-[var(--tx2)] break-all opacity-60">{retiredWallet}</p>
                    </div>
                  )}
                  <div className="p-3 border border-[var(--green)]/30 bg-[var(--green)]/5">
                    <p className="text-[8px] text-[var(--tx3)] uppercase font-bold mb-1 tracking-widest">Current Identity</p>
                    <p className="font-bold text-[11px] text-[var(--green)] break-all">{wallet?.pubkey ?? '—'}</p>
                  </div>
                </div>
                <button onClick={() => { setStep('intro') }}
                  className="w-full py-4 bg-[var(--bg2)] border border-[var(--bd)] text-white font-bold text-[10px] uppercase tracking-[2px] hover:bg-[var(--bg)] transition-all">
                  Return to Dashboard
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
