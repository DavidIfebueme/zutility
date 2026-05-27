import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "./types";
import { motion, useAnimate } from "motion/react";

const TimerOffIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".clock-face", { scale: [1, 1.08, 1] }, { duration: 0.4, ease: "easeInOut" });
      await animate(".slash", { opacity: [0.5, 1] }, { duration: 0.3, ease: "easeOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".clock-face, .slash", { scale: 1, opacity: 0.5 }, { duration: 0.2, ease: "easeInOut" });
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
        <motion.path className="clock-face" style={{ transformOrigin: "center" }} d="M12 12m-9 0a9 9 0 1 0 18 0a9 9 0 1 0 -18 0" />
        <motion.path d="M12 7v5l3 3" />
        <motion.line className="slash" x1="3" y1="3" x2="21" y2="21" opacity={0.5} />
      </motion.svg>
    );
  }
);

TimerOffIcon.displayName = "TimerOffIcon";
export default TimerOffIcon;
