import { ImageResponse } from 'next/og'

export const size = { width: 32, height: 32 }
export const contentType = 'image/png'

export default function Icon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          backgroundColor: '#0A0A0A',
          borderRadius: '6px',
        }}
      >
        <span
          style={{
            fontSize: '22px',
            fontWeight: 900,
            color: '#F4B728',
            lineHeight: 1,
          }}
        >
          z
        </span>
      </div>
    ),
    { ...size }
  )
}
