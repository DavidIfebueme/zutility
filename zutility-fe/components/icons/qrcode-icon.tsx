import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "./types";
import { motion, useAnimate } from "motion/react";

const QrcodeIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".outer-sq", { scale: [1, 0.92, 1] }, { duration: 0.4, ease: "easeInOut" });
      await animate(".inner-dot", { scale: [1, 0, 1.2, 1] }, { duration: 0.5, ease: "easeInOut" });
      animate(".scan-line", { y: [0, 8, 0] }, { duration: 0.5, ease: "easeInOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".outer-sq, .inner-dot, .scan-line", { scale: 1, y: 0 }, { duration: 0.2, ease: "easeInOut" });
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
          className="outer-sq"
          style={{ transformOrigin: "6px 6px" }}
          d="M4 4m0 2a2 2 0 0 1 2 -2h4a2 2 0 0 1 2 2v4a2 2 0 0 1 -2 2h-4a2 2 0 0 1 -2 -2z"
        />
        <motion.path className="inner-dot" style={{ transformOrigin: "6px 6px" }} d="M5 5l4 0" fill={color} />
        <motion.path
          className="outer-sq"
          style={{ transformOrigin: "18px 6px" }}
          d="M14 4m0 2a2 2 0 0 1 2 -2h4a2 2 0 0 1 2 2v4a2 2 0 0 1 -2 2h-4a2 2 0 0 1 -2 -2z"
        />
        <motion.path className="inner-dot" style={{ transformOrigin: "18px 6px" }} d="M15 5l4 0" fill={color} />
        <motion.path
          className="outer-sq"
          style={{ transformOrigin: "6px 18px" }}
          d="M4 14m0 2a2 2 0 0 1 2 -2h4a2 2 0 0 1 2 2v4a2 2 0 0 1 -2 2h-4a2 2 0 0 1 -2 -2z"
        />
        <motion.path className="inner-dot" style={{ transformOrigin: "6px 18px" }} d="M5 15l4 0" fill={color} />
        <motion.path className="scan-line" d="M14 14l6 0" />
        <motion.path d="M14 18l2 0" />
        <motion.path d="M18 18l2 0" />
        <motion.path d="M14 22l2 0" />
        <motion.path d="M18 22l2 0" />
      </motion.svg>
    );
  }
);

QrcodeIcon.displayName = "QrcodeIcon";
export default QrcodeIcon;
