import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const NineMobileIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(
        ".nine-circle",
        { scale: [1, 1.08, 1] },
        { duration: 0.5, ease: "easeInOut" },
      );
      animate(
        ".nine-digit",
        { y: [0, -1.5, 0] },
        { duration: 0.5, ease: "easeInOut" },
      );
    }, [animate]);

    const stop = useCallback(() => {
      animate(".nine-circle", { scale: 1 }, { duration: 0.2 });
      animate(".nine-digit", { y: 0 }, { duration: 0.2 });
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
          className="nine-circle"
          cx="12"
          cy="12"
          r="9"
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        />
        <motion.path
          className="nine-digit"
          d="M13 7a3 3 0 0 0-3 3c0 1.5 1 2.5 2 3l1 1-1 3"
          style={{ transformOrigin: "50% 50%", transformBox: "fill-box" }}
        />
      </motion.svg>
    );
  },
);

NineMobileIcon.displayName = "NineMobileIcon";
export default NineMobileIcon;
