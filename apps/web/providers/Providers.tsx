'use client'
import React from 'react'
import { QueryClient, QueryClientProvider } from 'react-query'
import { EngineProvider } from './EngineProvider'
import { SelfCustodyProvider } from './SelfCustodyProvider'

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: 2 } },
})

export function Providers({ children }: { children: React.ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <SelfCustodyProvider>
        <EngineProvider>
          {children}
        </EngineProvider>
      </SelfCustodyProvider>
    </QueryClientProvider>
  )
}
