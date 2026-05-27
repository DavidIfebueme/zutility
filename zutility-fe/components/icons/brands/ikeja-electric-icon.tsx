import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const IkejaElectricIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".bolt", { scale: [1, 1.12, 1], opacity: [0.7, 1, 1] }, { duration: 0.4, ease: "easeInOut" });
      await animate(".badge", { y: [-2, 0], opacity: [0.6, 1] }, { duration: 0.3, ease: "easeOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".bolt, .badge", { scale: 1, opacity: 1, y: 0 }, { duration: 0.2, ease: "easeInOut" });
    }, [animate]);

    useImperativeHandle(ref, () => ({ startAnimation: start, stopAnimation: stop }));

    return (
      <motion.svg
        ref={scope}
        xmlns="http://www.w3.org/2000/svg"
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        stroke={color}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
        className={`cursor-pointer ${className}`}
        onHoverStart={start}
        onHoverEnd={stop}
      >
        <motion.path
          className="bolt"
          d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"
          fill={color}
          stroke={color}
          opacity={0.7}
          style={{ transformOrigin: "center" }}
        />
        <motion.text
          className="badge"
          x="17"
          y="22"
          fontSize="6"
          fontWeight="bold"
          fill={color}
          textAnchor="middle"
          opacity={0.6}
        >
          I
        </motion.text>
      </motion.svg>
    );
  },
);

IkejaElectricIcon.displayName = "IkejaElectricIcon";
export default IkejaElectricIcon;
