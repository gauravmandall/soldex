import React, { useRef, useEffect } from 'react'
import { useHyperliquidCandles } from '@/hooks/useHyperliquidCandles'

interface Props { marketId: string; interval: string }

export function CandleChart({ marketId, interval }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const chartRef = useRef<any>(null)
  const seriesRef = useRef<any>(null)
  const { data, loading } = useHyperliquidCandles(marketId, interval)

  useEffect(() => {
    if (!containerRef.current) return
    
    let chart: any;
    
    import('lightweight-charts').then(({ createChart, ColorType, CrosshairMode }) => {
      if (!containerRef.current) return
      if (chartRef.current) { 
        chartRef.current.remove()
        chartRef.current = null 
      }

      chart = createChart(containerRef.current, {
        layout: { 
          background: { type: ColorType.Solid, color: '#0a0b0f' }, 
          textColor: '#8b90a8',
          fontSize: 11,
          fontFamily: "'Space Grotesk', sans-serif",
        },
        grid: { 
          vertLines: { color: 'rgba(30, 33, 48, 0.5)' }, 
          horzLines: { color: 'rgba(30, 33, 48, 0.5)' } 
        },
        crosshair: { 
          mode: CrosshairMode.Normal,
          vertLine: { color: '#4b5068', labelBackgroundColor: '#1e2130' },
          horzLine: { color: '#4b5068', labelBackgroundColor: '#1e2130' },
        },
        rightPriceScale: { 
          borderColor: '#1e2130',
          autoScale: true,
          alignLabels: true,
        },
        timeScale: { 
          borderColor: '#1e2130', 
          timeVisible: true, 
          secondsVisible: false,
          barSpacing: 8,
        },
        handleScroll: { vertTouchDrag: false },
        width: containerRef.current.clientWidth,
        height: containerRef.current.clientHeight,
      })

      const series = chart.addCandlestickSeries({
        upColor: '#22c55e', 
        downColor: '#ef4444',
        borderUpColor: '#22c55e', 
        borderDownColor: '#ef4444',
        wickUpColor: '#22c55e', 
        wickDownColor: '#ef4444',
        priceFormat: { type: 'price', precision: marketId.includes('SOL') ? 2 : 1, minMove: 0.01 },
      })

      chartRef.current = chart
      seriesRef.current = series

      const ro = new ResizeObserver(() => {
        if (containerRef.current && chartRef.current) {
          chartRef.current.applyOptions({ 
            width: containerRef.current.clientWidth, 
            height: containerRef.current.clientHeight 
          })
        }
      })
      ro.observe(containerRef.current)
      
      return () => { ro.disconnect(); chart.remove() }
    })
  }, [marketId]) // Only re-create chart if market changes, interval-driven history update happens via series.setData

  // Update data separately to avoid chart flickering
  useEffect(() => {
    if (seriesRef.current && data.length > 0) {
      seriesRef.current.setData(data)
    }
  }, [data])

  return (
    <div className="relative w-full h-full group">
      {loading && data.length === 0 && (
        <div className="absolute inset-0 z-10 flex items-center justify-center bg-[#0a0b0f]/50 backdrop-blur-sm">
          <div className="flex flex-col items-center gap-2">
            <div className="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
            <span className="text-[10px] text-[#4b5068] uppercase tracking-widest">Loading History</span>
          </div>
        </div>
      )}
      <div ref={containerRef} className="w-full h-full" />
    </div>
  )
}
