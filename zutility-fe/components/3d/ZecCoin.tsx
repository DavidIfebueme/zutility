"use client"

import { useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import { Environment, Float, Text } from '@react-three/drei'
import * as THREE from 'three'

export function ZecCoin() {
  const groupRef = useRef<THREE.Group>(null)

  useFrame((state) => {
    if (groupRef.current) {
      const t = state.clock.elapsedTime
      groupRef.current.rotation.z = Math.sin(t * 0.35) * 1.2
    }
  })

  return (
    <Float speed={1} rotationIntensity={0.05} floatIntensity={0.4}>
      <group ref={groupRef} scale={2.4}>
        <mesh rotation={[Math.PI / 2, 0, 0]}>
          <cylinderGeometry args={[1, 1, 0.12, 64]} />
          <meshStandardMaterial
            color="#F4B728"
            metalness={0.9}
            roughness={0.15}
            envMapIntensity={1.2}
          />
        </mesh>

        <mesh rotation={[Math.PI / 2, 0, 0]}>
          <cylinderGeometry args={[0.92, 0.92, 0.125, 64]} />
          <meshStandardMaterial
            color="#D9A21B"
            metalness={0.95}
            roughness={0.1}
            envMapIntensity={1}
          />
        </mesh>

        <mesh position={[0, 0.063, 0]} rotation={[Math.PI / 2, 0, 0]}>
          <torusGeometry args={[0.88, 0.03, 8, 64]} />
          <meshStandardMaterial
            color="#C4920F"
            metalness={0.95}
            roughness={0.05}
            envMapIntensity={1.5}
          />
        </mesh>

        <Text
          position={[0, 0.067, 0]}
          fontSize={1.15}
          textAlign="center"
          anchorX="center"
          anchorY="middle"
        >
          Z
          <meshStandardMaterial
            color="#FFF8E7"
            metalness={0.7}
            roughness={0.08}
            envMapIntensity={2}
          />
        </Text>

        <mesh position={[0, -0.063, 0]} rotation={[Math.PI / 2, 0, 0]}>
          <torusGeometry args={[0.88, 0.03, 8, 64]} />
          <meshStandardMaterial
            color="#C4920F"
            metalness={0.95}
            roughness={0.05}
            envMapIntensity={1.5}
          />
        </mesh>

        <Text
          position={[0, -0.067, 0]}
          fontSize={0.38}
          textAlign="center"
          anchorX="center"
          anchorY="middle"
          letterSpacing={0.18}
        >
          ZEC
          <meshStandardMaterial
            color="#FFF8E7"
            metalness={0.7}
            roughness={0.08}
            envMapIntensity={2}
          />
        </Text>
      </group>

      <mesh position={[0, 0, -1.2]}>
        <planeGeometry args={[8, 8]} />
        <meshBasicMaterial
          color="#F4B728"
          transparent
          opacity={0.04}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
    </Float>
  )
}

export function ZecCoinScene() {
  return (
    <>
      <ambientLight intensity={0.35} />
      <directionalLight position={[10, 10, 10]} intensity={1.3} color="#ffffff" />
      <directionalLight position={[-5, -5, 5]} intensity={0.25} color="#F4B728" />
      <pointLight position={[-10, -10, -10]} intensity={0.4} color="#F4B728" />
      <Environment preset="city" />
      <ZecCoin />
    </>
  )
}
