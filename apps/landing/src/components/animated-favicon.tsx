'use client';

import { useEffect, useRef } from 'react';

/**
 * AnimatedFavicon component implementing the "other method" from the blog post.
 * Instead of relying on a browser-buggy GIF, it uses a Canvas to draw frames
 * programmatically and updates the favicon href with a PNG Data URL.
 * 
 * It also uses a Web Worker to ensure the animation continues smoothly
 * even when the tab is in the background (preventing browser throttling).
 */
export function AnimatedFavicon() {
  const linkRef = useRef<HTMLLinkElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const imageRef = useRef<HTMLImageElement | null>(null);

  useEffect(() => {
    // 1. Get or create the favicon link element
    let link = document.querySelector('link[rel~="icon"]') as HTMLLinkElement;
    if (!link) {
      link = document.createElement('link');
      link.rel = 'icon';
      document.head.appendChild(link);
    }
    linkRef.current = link;

    // 2. Setup canvas
    const canvas = document.createElement('canvas');
    canvas.width = 32;
    canvas.height = 32;
    canvasRef.current = canvas;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return;

    // 3. Load the source image (using the GIF's first frame if available)
    const img = new Image();
    img.src = '/favicon.gif'; // Fallback logic is handled in updateFavicon
    imageRef.current = img;

    let frame = 0;
    const updateFavicon = () => {
      if (!ctx || !linkRef.current || !imageRef.current) return;

      const t = performance.now();
      ctx.clearRect(0, 0, 32, 32);

      // --- Animation Logic ---
      // We'll use a hue-rotate and a subtle scale pulse to make it feel "live"
      const hue = (t / 30) % 360;
      const pulse = 1 + 0.05 * Math.sin(t / 400);
      
      ctx.save();
      ctx.translate(16, 16);
      ctx.scale(pulse, pulse);
      ctx.translate(-16, -16);

      if (imageRef.current.complete && imageRef.current.naturalWidth > 0) {
        // Draw the image with a dynamic color shift
        ctx.filter = `hue-rotate(${hue}deg) saturate(1.2)`;
        ctx.drawImage(imageRef.current, 0, 0, 32, 32);
        
        // Add a small "active" dot in the corner
        ctx.filter = 'none';
        ctx.fillStyle = `hsl(${hue}, 80%, 60%)`;
        ctx.beginPath();
        ctx.arc(28, 4, 3, 0, Math.PI * 2);
        ctx.fill();
      } else {
        // Fallback: Elegant pulsing placeholder
        ctx.fillStyle = `hsl(${hue}, 70%, 60%)`;
        ctx.beginPath();
        ctx.roundRect(4, 4, 24, 24, 6);
        ctx.fill();
      }
      ctx.restore();

      // 4. Update the favicon href with a PNG Data URL (most compatible method)
      linkRef.current.href = canvas.toDataURL('image/png');
    };

    // 5. Web Worker Trick (for Background Tabs)
    // Browsers throttle requestAnimationFrame and setInterval in background tabs.
    // A Web Worker continues to run, allowing us to keep the favicon alive.
    const workerCode = `
      let timer;
      self.onmessage = (e) => {
        if (e.data === 'start') {
          timer = setInterval(() => self.postMessage('tick'), 150); // ~7 FPS is plenty for a favicon
        } else if (e.data === 'stop') {
          clearInterval(timer);
        }
      };
    `;
    const blob = new Blob([workerCode], { type: 'application/javascript' });
    const workerUrl = URL.createObjectURL(blob);
    const worker = new Worker(workerUrl);

    worker.onmessage = () => {
      updateFavicon();
    };
    worker.postMessage('start');

    // Also run a smooth loop when the tab is active
    let animationId: number;
    const loop = () => {
      updateFavicon();
      animationId = requestAnimationFrame(loop);
    };
    animationId = requestAnimationFrame(loop);

    return () => {
      cancelAnimationFrame(animationId);
      worker.postMessage('stop');
      worker.terminate();
      URL.revokeObjectURL(workerUrl);
    };
  }, []);

  return null;
}
