"use client"

import { useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import { Environment, Float, Text } from '@react-three/drei'
import * as THREE from 'three'

export function ZecCoin() {
  const groupRef = useRef<THREE.Group>(null)

  useFrame((state, delta) => {
    if (groupRef.current) {
      groupRef.current.rotation.y += delta * 0.6
    }
  })

  return (
    <Float speed={2} rotationIntensity={0.3} floatIntensity={0.8}>
      <group rotation={[Math.PI / 2, 0, 0]} scale={2}>
        <group ref={groupRef}>
          <mesh>
            <cylinderGeometry args={[1, 1, 0.12, 64]} />
            <meshStandardMaterial
              color="#F4B728"
              metalness={0.9}
              roughness={0.15}
              envMapIntensity={1.2}
            />
          </mesh>

          <mesh>
            <cylinderGeometry args={[0.92, 0.92, 0.125, 64]} />
            <meshStandardMaterial
              color="#D9A21B"
              metalness={0.95}
              roughness={0.1}
              envMapIntensity={1}
            />
          </mesh>

          <mesh position={[0, 0.063, 0]} rotation={[0, 0, 0]}>
            <torusGeometry args={[0.88, 0.03, 8, 64]} />
            <meshStandardMaterial
              color="#C4920F"
              metalness={0.95}
              roughness={0.05}
              envMapIntensity={1.5}
            />
          </mesh>

          <Text
            position={[0, 0.065, 0]}
            rotation={[0, 0, 0]}
            fontSize={1.1}
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

          <mesh position={[0, -0.063, 0]} rotation={[0, 0, 0]}>
            <torusGeometry args={[0.88, 0.03, 8, 64]} />
            <meshStandardMaterial
              color="#C4920F"
              metalness={0.95}
              roughness={0.05}
              envMapIntensity={1.5}
            />
          </mesh>

          <Text
            position={[0, -0.065, 0]}
            rotation={[0, Math.PI, 0]}
            fontSize={0.35}
            textAlign="center"
            anchorX="center"
            anchorY="middle"
            letterSpacing={0.15}
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
      </group>

      <mesh position={[0, 0, -1]}>
        <planeGeometry args={[6, 6]} />
        <meshBasicMaterial
          color="#F4B728"
          transparent
          opacity={0.06}
          blending={THREE.AdditiveBlending}
        />
      </mesh>
    </Float>
  )
}

export function ZecCoinScene() {
  return (
    <>
      <ambientLight intensity={0.4} />
      <directionalLight position={[10, 10, 10]} intensity={1.2} color="#ffffff" />
      <directionalLight position={[-5, -5, 5]} intensity={0.3} color="#F4B728" />
      <pointLight position={[-10, -10, -10]} intensity={0.5} color="#F4B728" />
      <Environment preset="city" />
      <ZecCoin />
    </>
  )
}
