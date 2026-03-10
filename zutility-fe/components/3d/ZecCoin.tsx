"use client"

import { useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import { useGLTF, Environment, Float, MeshDistortMaterial } from '@react-three/drei'
import * as THREE from 'three'

export function ZecCoin() {
  const meshRef = useRef<THREE.Mesh>(null)

  useFrame((state, delta) => {
    if (meshRef.current) {
      meshRef.current.rotation.y += delta * 0.5
      meshRef.current.rotation.x = Math.sin(state.clock.elapsedTime * 0.5) * 0.1
    }
  })

  return (
    <Float speed={2} rotationIntensity={0.5} floatIntensity={1}>
      <mesh ref={meshRef} scale={2}>
        <cylinderGeometry args={[1, 1, 0.1, 64]} />
        <meshStandardMaterial
          color="#F4B728"
          metalness={0.8}
          roughness={0.2}
          envMapIntensity={1}
        />
        {/* Z logo placeholder on the face */}
        <mesh position={[0, 0, 0.051]}>
          <planeGeometry args={[1, 1]} />
          <meshStandardMaterial
            color="#FFFFFF"
            metalness={0.5}
            roughness={0.1}
            transparent
            opacity={0.8}
          />
        </mesh>
      </mesh>
      
      {/* Subtle glow */}
      <mesh position={[0, 0, -0.5]}>
        <planeGeometry args={[5, 5]} />
        <meshBasicMaterial
          color="#F4B728"
          transparent
          opacity={0.1}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
    </Float>
  )
}

export function ZecCoinScene() {
  return (
    <>
      <ambientLight intensity={0.5} />
      <directionalLight position={[10, 10, 10]} intensity={1} color="#ffffff" />
      <pointLight position={[-10, -10, -10]} intensity={0.5} color="#F4B728" />
      <Environment preset="city" />
      <ZecCoin />
    </>
  )
}
