import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const DstvIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".dish", { rotate: [0, -5, 0] }, { duration: 0.5, ease: "easeInOut" });
      animate(".signal", { opacity: [0, 1, 0], x: [0, 2, 4] }, { duration: 0.6, ease: "easeOut" });
      await animate(".signal2", { opacity: [0, 1, 0], x: [0, 3, 6] }, { duration: 0.7, ease: "easeOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".dish, .signal, .signal2", { rotate: 0, opacity: 1, x: 0 }, { duration: 0.2, ease: "easeInOut" });
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
          className="dish"
          style={{ transformOrigin: "center bottom" }}
          d="M4 19C8 9 16 9 20 19"
        />
        <motion.path d="M12 19V12" />
        <motion.path d="M12 12L15 9" />
        <motion.path
          className="signal"
          d="M17 7a5 5 0 0 0-2-1"
        />
        <motion.path
          className="signal2"
          d="M19 5a8 8 0 0 0-3-2"
        />
      </motion.svg>
    );
  }
);

DstvIcon.displayName = "DstvIcon";
export default DstvIcon;
