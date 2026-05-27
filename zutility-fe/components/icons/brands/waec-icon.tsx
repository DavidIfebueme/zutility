import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const WaecIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(
        ".waec-shield",
        { rotate: [0, -3, 3, 0] },
        { duration: 0.5, ease: "easeInOut" },
      );
      animate(
        ".waec-letter",
        { scale: [1, 1.15, 1] },
        { duration: 0.5, ease: "easeInOut" },
      );
    }, [animate]);

    const stop = useCallback(() => {
      animate(".waec-shield", { rotate: 0 }, { duration: 0.2 });
      animate(".waec-letter", { scale: 1 }, { duration: 0.2 });
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
          className="waec-shield"
          d="M12 2l8 4v6c0 5-3.5 9.7-8 11-4.5-1.3-8-6-8-11V6l8-4z"
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        />
        <motion.text
          className="waec-letter"
          x="12"
          y="15"
          textAnchor="middle"
          fontSize="8"
          fontWeight="bold"
          fill={color}
          stroke="none"
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        >
          W
        </motion.text>
      </motion.svg>
    );
  },
);

WaecIcon.displayName = "WaecIcon";
export default WaecIcon;
