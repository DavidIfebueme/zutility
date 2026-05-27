import { forwardRef, useImperativeHandle, useCallback } from "react";
import type { AnimatedIconHandle, AnimatedIconProps } from "../types";
import { motion, useAnimate } from "motion/react";

const SchoolFeesIcon = forwardRef<AnimatedIconHandle, AnimatedIconProps>(
  ({ size = 24, color = "currentColor", strokeWidth = 2, className = "" }, ref) => {
    const [scope, animate] = useAnimate();

    const start = useCallback(async () => {
      animate(
        ".school-cap",
        { rotate: [0, -3, 3, 0] },
        { duration: 0.5, ease: "easeInOut" },
      );
      animate(
        ".school-tassel",
        { rotate: [0, -15, 15, 0] },
        { duration: 0.5, ease: "easeInOut" },
      );
    }, [animate]);

    const stop = useCallback(() => {
      animate(".school-cap", { rotate: 0 }, { duration: 0.2 });
      animate(".school-tassel", { rotate: 0 }, { duration: 0.2 });
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
        <motion.g
          className="school-cap"
          style={{ transformOrigin: "12px 10px", transformBox: "fill-box" }}
        >
          <motion.path d="M12 3l9 5-9 5-9-5 9-5z" />
          <motion.path d="M3 8v5l9 5 9-5V8" />
        </motion.g>
        <motion.g
          className="school-tassel"
          style={{ transformOrigin: "21px 8px", transformBox: "fill-box" }}
        >
          <motion.path d="M21 8v6" />
          <motion.circle cx="21" cy="15" r="1" />
        </motion.g>
      </motion.svg>
    );
  },
);

SchoolFeesIcon.displayName = "SchoolFeesIcon";
export default SchoolFeesIcon;
