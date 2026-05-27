"use client"

import { useRef } from 'react'
import { useFrame } from '@react-three/fiber'
import { Environment, Float, Text } from '@react-three/drei'
import * as THREE from 'three'

export function ZecCoin() {
  const groupRef = useRef<THREE.Group>(null)

  useFrame((state, delta) => {
    if (groupRef.current) {
      groupRef.current.rotation.y += delta * 0.5
      groupRef.current.rotation.x = Math.sin(state.clock.elapsedTime * 0.5) * 0.1
    }
  })

  return (
    <Float speed={2} rotationIntensity={0.5} floatIntensity={1}>
      <group ref={groupRef} scale={2}>
        <mesh>
          <cylinderGeometry args={[1, 1, 0.1, 64]} />
          <meshStandardMaterial
            color="#F4B728"
            metalness={0.9}
            roughness={0.15}
            envMapIntensity={1.2}
          />
        </mesh>

        <mesh>
          <cylinderGeometry args={[0.92, 0.92, 0.102, 64]} />
          <meshStandardMaterial
            color="#D9A21B"
            metalness={0.95}
            roughness={0.1}
            envMapIntensity={1}
          />
        </mesh>

        <mesh position={[0, 0, 0.052]} rotation={[-Math.PI / 2, 0, 0]}>
          <torusGeometry args={[0.88, 0.03, 8, 64]} />
          <meshStandardMaterial
            color="#C4920F"
            metalness={0.95}
            roughness={0.05}
            envMapIntensity={1.5}
          />
        </mesh>

        <Text
          position={[0, 0.06, 0.052]}
          rotation={[-Math.PI / 2, 0, 0]}
          fontSize={1.1}
          maxWidth={1}
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

        <mesh position={[0, 0, -0.052]} rotation={[Math.PI / 2, 0, Math.PI]}>
          <torusGeometry args={[0.88, 0.03, 8, 64]} />
          <meshStandardMaterial
            color="#C4920F"
            metalness={0.95}
            roughness={0.05}
            envMapIntensity={1.5}
          />
        </mesh>

        <Text
          position={[0, -0.06, -0.052]}
          rotation={[Math.PI / 2, 0, Math.PI]}
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

      <mesh position={[0, 0, -0.5]}>
        <planeGeometry args={[5, 5]} />
        <meshBasicMaterial
          color="#F4B728"
          transparent
          opacity={0.08}
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
