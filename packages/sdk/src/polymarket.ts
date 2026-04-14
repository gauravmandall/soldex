/**
 * Polymarket integration utilities.
 * Soldex engine handles the heavy lifting; this SDK provides
 * client-side helpers for constructing Poly orders.
 */

export interface PolyOrder {
  tokenId: string      // YES or NO token
  side: 'BUY' | 'SELL'
  price: number        // 0.0–1.0 (probability / dollar)
  size: number         // USDC amount
  expiration?: number  // unix timestamp, undefined = GTC
}

/** Compute implied probability from best bid/ask */
export function impliedProbability(bid: number, ask: number): number {
  return (bid + ask) / 2
}

/** Compute expected value of a bet */
export function expectedValue(
  probability: number,
  price: number,
  payout: number = 1.0,
): number {
  return probability * (payout - price) - (1 - probability) * price
}

/** Format probability as percentage string */
export function fmtProbability(p: number): string {
  return `${(p * 100).toFixed(1)}%`
}
