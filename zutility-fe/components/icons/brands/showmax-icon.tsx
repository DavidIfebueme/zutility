import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const ShowmaxIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".play", { x: [0, 1, 0] }, { duration: 0.4, ease: "easeInOut" });
      await animate(".screen", { strokeWidth: [2, 2.5, 2] }, { duration: 0.4, ease: "easeInOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".play, .screen", { x: 0, strokeWidth: 2 }, { duration: 0.2, ease: "easeInOut" });
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
          className="screen"
          d="M3 7a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"
        />
        <motion.path
          className="play"
          d="M10 8l6 4-6 4V8z"
        />
      </motion.svg>
    );
  }
);

ShowmaxIcon.displayName = "ShowmaxIcon";
export default ShowmaxIcon;
