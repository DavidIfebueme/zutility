import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const StartimesIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(".star", { rotate: [0, 15, 0], scale: [1, 1.1, 1] }, { duration: 0.5, ease: "easeInOut" });
      await animate(".screen", { opacity: [1, 0.7, 1] }, { duration: 0.4, ease: "easeInOut" });
    }, [animate]);

    const stop = useCallback(() => {
      animate(".star, .screen", { rotate: 0, scale: 1, opacity: 1 }, { duration: 0.2, ease: "easeInOut" });
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
          className="star"
          style={{ transformOrigin: "center" }}
          d="M12 2l2.4 7.2H22l-6 4.4 2.3 7.2L12 16.4l-6.3 4.4 2.3-7.2-6-4.4h7.6z"
        />
      </motion.svg>
    );
  }
);

StartimesIcon.displayName = "StartimesIcon";
export default StartimesIcon;
