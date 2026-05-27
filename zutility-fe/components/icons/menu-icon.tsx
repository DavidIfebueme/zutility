import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "./types";
import { motion, useAnimate } from "motion/react";

const MenuIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".line-top", { x: [0, 2] }, { duration: 0.3, ease: "easeOut" });
      animate(".line-mid", { scaleX: [1, 1.1, 1] }, { duration: 0.35, ease: "easeInOut" });
      await animate(".line-bot", { x: [0, -2] }, { duration: 0.3, ease: "easeOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".line-top, .line-mid, .line-bot", { x: 0, scaleX: 1 }, { duration: 0.2, ease: "easeInOut" });
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
        <motion.path className="line-top" d="M4 6h16" />
        <motion.path className="line-mid" style={{ transformOrigin: "center" }} d="M4 12h16" />
        <motion.path className="line-bot" d="M4 18h16" />
      </motion.svg>
    );
  }
);

MenuIcon.displayName = "MenuIcon";
export default MenuIcon;
