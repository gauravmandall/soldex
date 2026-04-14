export interface MarketConfig {
  id: string
  base: string
  quote: string
  programId: string
  priceFeed: string
  maxLeverage: number
  tickSize: number
  lotSize: number
}

export interface PerpsOrder {
  marketId: string
  side: 'long' | 'short'
  size: number
  leverage: number
  collateralUsdc: number
  orderType: 'market' | 'limit'
  limitPrice?: number
}

export interface PerpsPosition {
  marketId: string
  owner: string
  side: 'long' | 'short'
  size: number
  entryPrice: number
  collateral: number
  leverageBps: number
  openedAt: number
}

export type Network = 'mainnet-beta' | 'devnet' | 'localnet'
