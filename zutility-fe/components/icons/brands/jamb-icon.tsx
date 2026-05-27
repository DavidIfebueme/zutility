import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const JambIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(
        ".jamb-doc",
        { y: [0, -2, 0] },
        { duration: 0.5, ease: "easeInOut" },
      );
      animate(
        ".jamb-letter",
        { opacity: [0.7, 1, 0.7] },
        { duration: 0.5, ease: "easeInOut" },
      );
      animate(
        ".jamb-fold",
        { scale: [1, 1.15, 1] },
        { duration: 0.5, ease: "easeInOut" },
      );
    }, [animate]);

    const stop = useCallback(() => {
      animate(".jamb-doc", { y: 0 }, { duration: 0.2 });
      animate(".jamb-letter", { opacity: 0.7 }, { duration: 0.2 });
      animate(".jamb-fold", { scale: 1 }, { duration: 0.2 });
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
        <motion.g className="jamb-doc">
          <motion.path d="M6 4h12a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z" />
          <motion.path d="M8 18h8" />
        </motion.g>
        <motion.path
          className="jamb-fold"
          d="M14 2v4h4"
          style={{ transformOrigin: "14px 2px", transformBox: "fill-box" }}
        />
        <motion.text
          className="jamb-letter"
          x="12"
          y="16"
          textAnchor="middle"
          fontSize="9"
          fontWeight="bold"
          fill={color}
          stroke="none"
          opacity={0.7}
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        >
          J
        </motion.text>
      </motion.svg>
    );
  },
);

JambIcon.displayName = "JambIcon";
export default JambIcon;
