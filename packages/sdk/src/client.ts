import { AnchorProvider, Program, Idl, setProvider } from '@coral-xyz/anchor'
import { Connection, PublicKey, Keypair } from '@solana/web3.js'
import type { Network } from './types'

const PROGRAM_IDS: Record<Network, string> = {
  'mainnet-beta': 'REPLACE_WITH_MAINNET_ID',
  'devnet':       'REPLACE_WITH_DEVNET_ID',
  'localnet':     'Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS',
}

export class SoldexClient {
  readonly connection: Connection
  readonly programId:  PublicKey
  readonly provider:   AnchorProvider
  readonly network:    Network

  constructor(opts: {
    rpcUrl: string
    network?: Network
    wallet?: { publicKey: PublicKey; signTransaction: (tx: any) => Promise<any>; signAllTransactions: (txs: any[]) => Promise<any[]> }
  }) {
    this.network  = opts.network ?? 'devnet'
    this.connection = new Connection(opts.rpcUrl, 'confirmed')
    this.programId  = new PublicKey(PROGRAM_IDS[this.network])

    const wallet = opts.wallet ?? {
      publicKey: PublicKey.default,
      signTransaction: async (tx: any) => tx,
      signAllTransactions: async (txs: any[]) => txs,
    }

    this.provider = new AnchorProvider(this.connection, wallet as any, {
      commitment: 'confirmed',
      preflightCommitment: 'confirmed',
    })
    setProvider(this.provider)
  }

  /** Derive PDA for a market */
  async marketPDA(marketId: string): Promise<[PublicKey, number]> {
    const enc = new TextEncoder()
    return PublicKey.findProgramAddressSync(
      [Buffer.from('market'), Buffer.from(enc.encode(marketId))],
      this.programId
    )
  }

  /** Derive PDA for user margin account */
  async marginPDA(marketId: string, userPubkey: PublicKey): Promise<[PublicKey, number]> {
    const enc = new TextEncoder()
    return PublicKey.findProgramAddressSync(
      [Buffer.from('margin'), Buffer.from(enc.encode(marketId)), userPubkey.toBuffer()],
      this.programId
    )
  }

  /** Derive PDA for a position */
  async positionPDA(marketId: string, userPubkey: PublicKey, nonce: number): Promise<[PublicKey, number]> {
    const enc = new TextEncoder()
    return PublicKey.findProgramAddressSync(
      [Buffer.from('position'), Buffer.from(enc.encode(marketId)), userPubkey.toBuffer(), Buffer.from([nonce])],
      this.programId
    )
  }
}
