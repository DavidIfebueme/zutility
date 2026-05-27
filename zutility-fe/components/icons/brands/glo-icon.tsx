import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const GloIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(
        ".glo-circle",
        { rotate: [0, 10, 0] },
        { duration: 0.5, ease: "easeInOut" },
      );
      animate(
        ".glo-letter",
        { scale: [1, 1.12, 1] },
        { duration: 0.5, ease: "easeInOut" },
      );
    }, [animate]);

    const stop = useCallback(() => {
      animate(".glo-circle", { rotate: 0 }, { duration: 0.2 });
      animate(".glo-letter", { scale: 1 }, { duration: 0.2 });
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
        <motion.circle
          className="glo-circle"
          cx="12"
          cy="12"
          r="9"
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        />
        <motion.path
          className="glo-letter"
          d="M15 10h-3a3 3 0 1 0 2.5 4.5"
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        />
      </motion.svg>
    );
  },
);

GloIcon.displayName = "GloIcon";
export default GloIcon;
