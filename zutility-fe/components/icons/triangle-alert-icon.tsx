import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "./types";
import { motion, useAnimate } from "motion/react";

const TriangleAlertIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".triangle", { scale: [1, 1.1, 1] }, { duration: 0.4, ease: "easeInOut" });
      await animate(".exclaim", { y: [-1, 0.5, 0], opacity: [0.7, 1, 1] }, { duration: 0.35, ease: "easeOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".triangle, .exclaim", { scale: 1, y: 0, opacity: 1 }, { duration: 0.2, ease: "easeInOut" });
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
        <path stroke="none" d="M0 0h24v24H0z" fill="none" />
        <motion.path
          className="triangle"
          style={{ transformOrigin: "50% 60%" }}
          d="M12 1.674l9.87 17.073a1.3 1.3 0 0 1 -1.132 1.953h-19.476a1.3 1.3 0 0 1 -1.132 -1.953l9.87 -17.073z"
        />
        <motion.path className="exclaim" d="M12 9v4" />
        <motion.path className="exclaim" d="M12 17h.01" />
      </motion.svg>
    );
  }
);

TriangleAlertIcon.displayName = "TriangleAlertIcon";
export default TriangleAlertIcon;
