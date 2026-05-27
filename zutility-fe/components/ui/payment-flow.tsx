"use client"

import * as React from "react"
import { motion } from "motion/react"
import PhoneVolume from "@/components/icons/phone-volume"
import TvIcon from "@/components/icons/tv-icon"
import PlugConnectedIcon from "@/components/icons/plug-connected-icon"
import BookIcon from "@/components/icons/book-icon"

const PARTICLES = Array.from({ length: 24 }, (_, i) => ({
  id: i,
  delay: i * 0.12,
  duration: 2 + Math.random() * 1.5,
}))

const UTILITY_NODES: { Ico: React.ComponentType<any>; x: number; y: number; label: string }[] = [
  { Ico: PhoneVolume, label: "Airtime", x: -120, y: 80 },
  { Ico: TvIcon, label: "TV", x: 0, y: 100 },
  { Ico: PlugConnectedIcon, label: "Electricity", x: 120, y: 80 },
  { Ico: BookIcon, label: "Education", x: -60, y: 130 },
]

function Particle({ delay, duration }: { delay: number; duration: number }) {
  return (
    <motion.circle
      cx={0}
      cy={0}
      r={2}
      fill="#F4B728"
      opacity={0}
      initial={{ cx: 0, cy: 0, opacity: 0 }}
      animate={{
        cx: [0, 0],
        cy: [0, 60 + Math.random() * 20],
        opacity: [0, 0.8, 0],
      }}
      transition={{
        delay,
        duration,
        repeat: Infinity,
        repeatDelay: 3,
        ease: "easeOut",
      }}
    />
  )
}

function UtilityNode({ Ico, x, y }: typeof UTILITY_NODES[0]) {
  const [hovered, setHovered] = React.useState(false)

  return (
    <g
      style={{ transform: `translate(${x}px, ${y}px)` }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <circle r={32} fill="#F4B728" opacity={0.08} />
      <motion.circle
        r={26}
        fill="#F4B728"
        animate={{
          scale: hovered ? [1.15, 1] : [1, 1.15],
          opacity: hovered ? [0.25, 0.12] : [0.12, 0.25],
        }}
        transition={{ type: "spring", stiffness: 300, damping: 20 }}
      />
      <foreignObject x="-18" y="-14" width="36" height="28">
        <div className="flex items-center justify-center h-full">
          <Ico size={16} color="#FFF8E7" />
        </div>
      </foreignObject>
    </g>
  )
}

export function PaymentFlow() {
  return (
    <svg viewBox="0 0 400 400" className="w-full h-full max-w-[500px]" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <radialGradient id="zec-glow" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#F4B728" stopOpacity={0.3} />
          <stop offset="50%" stopColor="#F4B728" stopOpacity={0.08} />
          <stop offset="100%" stopColor="#F4B728" stopOpacity={0} />
        </radialGradient>
        <linearGradient id="line-grad" x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#F4B728" stopOpacity={0} />
          <stop offset="20%" stopColor="#F4B728" stopOpacity={0.6} />
          <stop offset="80%" stopColor="#F4B728" stopOpacity={0.3} />
          <stop offset="100%" stopColor="#F4B728" stopOpacity={0} />
        </linearGradient>
        <filter id="glow">
          <feGaussianBlur stdDeviation="4" result="blur" />
          <feComposite in="SourceGraphic" in2="blur" operator="over" />
        </filter>
      </defs>

      <g transform="translate(200, 140)">
        <motion.g
          animate={{ rotate: [0, 360] }}
          transition={{ duration: 20, repeat: Infinity, ease: "linear" }}
        >
          <circle r={38} fill="url(#zec-glow)" filter="url(#glow)" />
          <text
            textAnchor="middle"
            dominantBaseline="central"
            fontSize={34}
            fontWeight={800}
            fontFamily="system-ui, sans-serif"
            fill="#FFF8E7"
            letterSpacing={-1}
          >
            Z
          </text>
        </motion.g>

        {PARTICLES.map((p) => (
          <Particle key={p.id} delay={p.delay} duration={p.duration} />
        ))}

        <path
          d="M 0 40 L 0 180"
          stroke="url(#line-grad)"
          strokeWidth={1.5}
          strokeLinecap="round"
          strokeDasharray="6 8"
          opacity={0.5}
        >
          <animate
            attributeName="stroke-dashoffset"
            from="0"
            to="-28"
            dur="2s"
            repeatCount="indefinite"
          />
        </path>

        <path
          d="M 0 220 C -80 240, -160 260, -200 280 M 0 220 C 80 240, 160 260, 200 280"
          stroke="#F4B728"
          strokeWidth={1}
          strokeLinecap="round"
          opacity={0.25}
          fill="none"
        />

        {UTILITY_NODES.map((u) => (
          <UtilityNode key={u.label} {...u} />
        ))}
      </g>
    </svg>
  )
}
