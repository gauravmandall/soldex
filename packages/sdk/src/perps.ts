import { PublicKey, Transaction, SystemProgram } from '@solana/web3.js'
import type { SoldexClient } from './client'
import type { PerpsOrder, PerpsPosition } from './types'

export class PerpsSDK {
  constructor(private client: SoldexClient) {}

  /**
   * Build an unsigned Transaction to open a position.
   * The frontend signs it using the self-custody keypair.
   */
  async buildOpenPositionTx(
    owner: PublicKey,
    order: PerpsOrder,
  ): Promise<Transaction> {
    const [marketPDA] = await this.client.marketPDA(order.marketId)
    const [marginPDA] = await this.client.marginPDA(order.marketId, owner)
    const nonce = Math.floor(Math.random() * 255)
    const [positionPDA] = await this.client.positionPDA(order.marketId, owner, nonce)

    // In production: use program.methods.openPosition(...).accounts({...}).transaction()
    // This stub returns an empty tx as placeholder
    const tx = new Transaction()
    tx.recentBlockhash = (
      await this.client.connection.getLatestBlockhash()
    ).blockhash
    tx.feePayer = owner
    return tx
  }

  /** Fetch all positions for a wallet */
  async fetchPositions(owner: PublicKey): Promise<PerpsPosition[]> {
    // In production: getProgramAccounts with memcmp filter on owner field
    return []
  }

  /** Fetch mark price from on-chain Pyth */
  async fetchMarkPrice(marketId: string): Promise<number> {
    // Stub — replace with Pyth SDK or Jupiter price API
    const prices: Record<string, number> = {
      'SOL-PERP': 150.0,
      'BTC-PERP': 65000.0,
      'ETH-PERP': 3500.0,
    }
    return prices[marketId] ?? 0
  }
}
