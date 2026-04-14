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
          background: { type: ColorType.Solid, color: '#0d1117' }, 
          textColor: '#94a3b8',
          fontSize: 10,
          fontFamily: "'JetBrains Mono', monospace",
        },
        grid: { 
          vertLines: { color: 'rgba(30, 38, 52, 0.4)' }, 
          horzLines: { color: 'rgba(30, 38, 52, 0.4)' } 
        },
        crosshair: { 
          mode: CrosshairMode.Normal,
          vertLine: { color: '#3b82f6', labelBackgroundColor: '#1e3a5f', width: 1, style: 2 },
          horzLine: { color: '#3b82f6', labelBackgroundColor: '#1e3a5f', width: 1, style: 2 },
        },
        rightPriceScale: { 
          borderColor: '#1e2634',
          autoScale: true,
          alignLabels: true,
          scaleMargins: { top: 0.1, bottom: 0.2 },
        },
        timeScale: { 
          borderColor: '#1e2634', 
          timeVisible: true, 
          secondsVisible: false,
          barSpacing: 10,
        },
        handleScroll: { vertTouchDrag: false },
        width: containerRef.current.clientWidth,
        height: containerRef.current.clientHeight,
      })

      const series = chart.addCandlestickSeries({
        upColor: '#10b981', 
        downColor: '#ef4444',
        borderVisible: false,
        wickUpColor: '#10b981', 
        wickDownColor: '#ef4444',
        priceFormat: { type: 'price', precision: 3, minMove: 0.001 },
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
