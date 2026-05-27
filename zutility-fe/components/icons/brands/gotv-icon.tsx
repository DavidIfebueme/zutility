import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const GotvIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".antenna", { scaleY: [1, 1.2, 1] }, { duration: 0.4, ease: "easeInOut" });
      animate(".screen", { opacity: [1, 0.6, 1] }, { duration: 0.5, ease: "easeInOut" });
      await animate(".g-mark", { scale: [1, 1.1, 1] }, { duration: 0.4, ease: "easeInOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".antenna, .screen, .g-mark", { scaleY: 1, opacity: 1, scale: 1 }, { duration: 0.2, ease: "easeInOut" });
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
          className="antenna"
          style={{ transformOrigin: "center bottom" }}
          d="M8 6L6 2"
        />
        <motion.path
          className="antenna"
          style={{ transformOrigin: "center bottom" }}
          d="M16 6l2-4"
        />
        <motion.path
          className="screen"
          d="M3 8a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8z"
        />
        <motion.path
          className="g-mark"
          style={{ transformOrigin: "12px 13px" }}
          d="M14 12h-2a2 2 0 1 0 1.5 3"
        />
      </motion.svg>
    );
  }
);

GotvIcon.displayName = "GotvIcon";
export default GotvIcon;
