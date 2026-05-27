import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "./types";
import { motion, useAnimate } from "motion/react";

const TvIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".screen", { scale: [1, 1.04, 1] }, { duration: 0.4, ease: "easeInOut" });
      animate(".signal", { y: [-3, 0], opacity: [0, 1] }, { duration: 0.3, ease: "easeOut" });
      await animate(".stand", { scaleY: [1, 1.15, 1] }, { duration: 0.35, ease: "easeInOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".screen, .signal, .stand", { scale: 1, scaleY: 1, y: 0, opacity: 1 }, { duration: 0.2, ease: "easeInOut" });
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
          className="screen"
          style={{ transformOrigin: "center" }}
          d="M3 7m0 2a2 2 0 0 1 2 -2h14a2 2 0 0 1 2 2v9a2 2 0 0 1 -2 2h-14a2 2 0 0 1 -2 -2z"
        />
        <motion.path className="signal" d="M16 3l-4 4l-4 -4" />
        <motion.path className="stand" style={{ transformOrigin: "center bottom" }} d="M7 20l10 0" />
        <motion.path className="stand" style={{ transformOrigin: "center bottom" }} d="M12 16l0 4" />
      </motion.svg>
    );
  }
);

TvIcon.displayName = "TvIcon";
export default TvIcon;
